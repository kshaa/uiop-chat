use std::{io, sync::{mpsc::{self, Receiver, Sender}, Arc}, thread, time::Duration};

use anyhow::{anyhow, Context};
use log::*;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::Mutex;
use tui_logger::*;
use crate::{client::{DspReader, DspWriter}, config::DspClientConfig, logger::NS_CHAT, protocol::{DspMessage, DspPayload, MessageMessage}};
use self::crossterm_backend::*;

pub struct App {
    client_reader: Option<DspReader>,
    client_writer: Arc<Mutex<Option<DspWriter>>>,
    client_config: DspClientConfig,
    app_event_rx: Option<Receiver<AppEvent>>,
    app_event_tx: Sender<AppEvent>,
    mode: AppMode,
    states: Vec<TuiWidgetState>,
    tab_names: Vec<&'static str>,
    selected_tab: usize,
    active_message: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum AppMode {
    #[default]
    Run,
    Quit,
}

enum AppEvent {
    UiEvent(Event),
    PayloadReceived(DspPayload),
    PayloadSent((DspWriter, String, String)),
}

impl App {
    pub fn new(client_reader: DspReader, client_writer: DspWriter, client_config: DspClientConfig) -> App {
        let (app_event_tx, app_event_rx) = mpsc::channel::<AppEvent>();

        let states = vec![
            TuiWidgetState::new().set_default_display_level(LevelFilter::Info),
            TuiWidgetState::new().set_default_display_level(LevelFilter::Info),
        ];

        // Adding this line had provoked the bug as described in issue #69
        // let states = states.into_iter().map(|s| s.set_level_for_target("some::logger", LevelFilter::Off)).collect();
        let tab_names = vec!["Message", "Quit"];
        App {
            client_reader: Some(client_reader),
            client_writer: Arc::new(Mutex::new(Some(client_writer))),
            client_config,
            app_event_tx,
            app_event_rx: Some(app_event_rx),
            mode: AppMode::Run,
            states,
            tab_names,
            selected_tab: 0,
            active_message: String::from(""),
        }
    }

    pub fn start(mut self, terminal: &mut Terminal<impl Backend>) -> anyhow::Result<()> {
        // Use an mpsc::channel to combine stdin events with app events
        let event_rx = self.app_event_rx.take().ok_or(anyhow!("App initialized without UI event receiver"))?;
        let event_tx = self.app_event_tx.clone();
        let payload_receive_tx = self.app_event_tx.clone();
        let client_reader = self.client_reader.take().ok_or(anyhow!("App initialized without DSP reader"))?;

        thread::spawn(move || input_thread(event_tx));
        tokio::spawn(async move {
            payload_receive_task(client_reader, payload_receive_tx).await.unwrap()
        });

        self.run(terminal, event_rx)
    }

    pub fn start_with_crossterm(self) -> anyhow::Result<()> {
        let mut terminal = init_terminal()?;
        terminal.clear()?;
        terminal.hide_cursor()?;
    
        self.start(&mut terminal)?;

        restore_terminal()?;
        terminal.clear()?;    
        
        Ok(())
    }

    /// Main application loop
    fn run(
        &mut self,
        terminal: &mut Terminal<impl Backend>,
        rx: mpsc::Receiver<AppEvent>,
    ) -> anyhow::Result<()> {
        for event in rx {
            match event {
                AppEvent::UiEvent(event) => self.handle_ui_event(event),
                AppEvent::PayloadReceived(payload) => self.react_to_payload(payload),
                AppEvent::PayloadSent((writer, username, text)) => self.active_message_sent(writer, username, text),
            }
            if self.mode == AppMode::Quit {
                break;
            }
            self.draw(terminal)?;
        }
        Ok(())
    }

    fn react_to_payload(&mut self, payload: DspPayload) {
        let username = payload.username;
        let message = payload.message;
        match message {
            DspMessage::JoinMessage(_) => debug!(target: NS_CHAT, "User '{}' has joined the server", username),
            DspMessage::QuitMessage(_) => debug!(target: NS_CHAT, "User '{}' has left the server", username),
            DspMessage::MessageMessage(m) => info!(target: NS_CHAT, "[{}] {}", username, m.text),
            DspMessage::ChallengeMessage(_) => error!(target: NS_CHAT, "You've received a rate-limiting challenge which is not implemented. It's left unimplemented, you might get disconnected."),
            DspMessage::RescindedMessage(_) => warn!(target: NS_CHAT, "The challenge has been rescinded, you can chat again"),
            DspMessage::ResponseMessage(_) => warn!(target: NS_CHAT, "You've received a challenge response, this shouldn't happen. Inform server admin."),
            DspMessage::ErrorMessage(m) => error!(target: NS_CHAT, "Server error: {}", m.text),
        }
    }

    fn handle_ui_event(&mut self, event: Event) {
        debug!(target: "App", "Handling UI event: {:?}",event);
        let selected_tab = self.selected_tab;
        let state = self.selected_state();

        if let Event::Key(key) = event {
            let code = key.code;

            match code.into() {
                // Tab switching
                Key::Char('\t') => self.next_tab(),
                Key::Tab => self.next_tab(),

                // Chat scrolling
                Key::Esc => state.transition(TuiWidgetEvent::EscapeKey),
                Key::PageUp => state.transition(TuiWidgetEvent::PrevPageKey),
                Key::PageDown => state.transition(TuiWidgetEvent::NextPageKey),

                // Message sending
                Key::Char(c) if selected_tab == 0 => self.add_active_message(c),
                Key::Backspace if selected_tab == 0 => self.backspace_active_message(),
                Key::Enter if selected_tab == 0 => self.send_active_message(),

                // Quitting
                Key::Enter if selected_tab == 1 => self.mode = AppMode::Quit,

                _ => (),
            }
        }
    }

    fn selected_state(&mut self) -> &mut TuiWidgetState {
        &mut self.states[self.selected_tab]
    }

    fn next_tab(&mut self) {
        self.selected_tab = (self.selected_tab + 1) % self.tab_names.len();
    }

    fn add_active_message(&mut self, c: char) {
        self.active_message.push(c);
    }

    fn backspace_active_message(&mut self) {
        self.active_message.pop();
    }

    // Quite unsafe
    fn send_active_message(&mut self) {
        let ui_tx = self.app_event_tx.clone();
        let username = self.client_config.username.clone();
        let text = self.active_message.clone();
        if text.is_empty() {
            error!(target: NS_CHAT, "Can't send empty message");
            return
        }
        let message = DspMessage::MessageMessage(MessageMessage { text: text.clone() });
        let payload = DspPayload { username: username.clone(), message };
        let mut client_writer = match self.client_writer.try_lock() {
            Ok(mut maybe_writer) => {
                match maybe_writer.take() {
                    Some(writer) => writer,
                    None => {
                        error!(target: NS_CHAT, "Failed to send message, another message might already be in-transit");
                        return
                    },
                }
            },
            Err(_) => {
                error!(target: NS_CHAT, "Failed to send message, another message is about to possibly go in-transit");
                return
            },
        };

        tokio::spawn(async move {
            client_writer.write(payload).await.with_context(|| format!("Failed to send message to server")).unwrap();
            let message = AppEvent::PayloadSent((client_writer, username, text));
            ui_tx.send(message).with_context(|| format!("Failed to return DspWriter to UI")).unwrap();
        });
    }

    fn active_message_sent(&mut self, writer: DspWriter, username: String, text: String) {
        loop {
            match self.client_writer.try_lock() {
                Ok(mut parking) => { 
                    // Got access to mutex, left writer back in app
                    let _ = parking.insert(writer);
                    self.active_message = String::from("");
                    return 
                },
                _ => {
                    // Couldn't get access to mutex, retry later?
                    thread::sleep(Duration::from_millis(100));
                    continue
                }
            }    
        }
    }

    fn draw(&mut self, terminal: &mut Terminal<impl Backend>) -> anyhow::Result<()> {
        terminal.draw(|frame| {
            frame.render_widget(self, frame.area());
        })?;
        Ok(())
    }
}

async fn payload_receive_task(mut reader: DspReader, tx: mpsc::Sender<AppEvent>) -> anyhow::Result<()> {
    loop {
        let payload = reader.read().await?;
        tx.send(AppEvent::PayloadReceived(payload)).with_context(|| format!("Failed to send received DSP message to UI"))?;
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [tabs_area, smart_area, prompt_area, help_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(50),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .areas(area);

        Tabs::new(self.tab_names.iter().cloned())
            .block(Block::default().title("States").borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .select(self.selected_tab)
            .render(tabs_area, buf);

        let filter_state = TuiWidgetState::new()
            .set_default_display_level(LevelFilter::Debug);

        TuiLoggerWidget::default()
            .block(Block::default().title(format!("Server chat: {}", self.client_config.server_address)).borders(Borders::ALL))
            .style_error(Style::default().fg(Color::Red).italic())
            .style_debug(Style::default().fg(Color::Gray).italic())
            .style_warn(Style::default().fg(Color::Yellow).italic())
            .style_trace(Style::default().fg(Color::Gray).italic())
            .style_info(Style::default().fg(Color::White))
            .output_separator(' ')
            .output_timestamp(Some("%H:%M:%S".to_string()))
            .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
            .output_target(true)
            .output_file(false)
            .output_line(false)
            .state(&filter_state)
            .render(smart_area, buf);

        let prompt_block = Block::new()
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .style(Style::default())
            .title(format!("[{}]", self.client_config.username));

        let prompt_block_inner = prompt_block.inner(prompt_area);
        prompt_block.render(prompt_area, buf);
    
        let style = Style::new().white();
        Text::raw(self.active_message.clone()).style(style).render(prompt_block_inner, buf);

        if area.width > 40 {
            Text::from(vec![
                "Tab: Switch state | Enter: Trigger state".into(),
                "PageUp/Down: Scroll | Esc: Cancel scroll".into(),
            ])
            .style(Color::Gray)
            .centered()
            .render(help_area, buf);
        }
    }
}

pub mod crossterm_backend {
    use super::*;

    pub use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode as Key},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };

    pub fn init_terminal() -> io::Result<Terminal<impl Backend>> {
        trace!(target:"crossterm", "Initializing terminal");
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(io::stdout());
        Terminal::new(backend)
    }

    pub fn restore_terminal() -> io::Result<()> {
        trace!(target:"crossterm", "Restoring terminal");
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)
    }

    pub fn input_thread(tx_event: mpsc::Sender<AppEvent>) -> anyhow::Result<()> {
        trace!(target:"crossterm", "Starting input thread");
        while let Ok(event) = event::read() {
            trace!(target:"crossterm", "Stdin event received {:?}", event);
            tx_event.send(AppEvent::UiEvent(event))?;
        }
        Ok(())
    }
}
