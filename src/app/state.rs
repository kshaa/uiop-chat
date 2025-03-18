use std::{
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

use super::{event::AppEvent, mode::AppMode};
use crate::app::crossterm_backend::*;
use crate::{
    client::{DspReader, DspWriter},
    config::DspClientConfig,
    logger::{NS_APP, NS_CHAT},
    protocol::{DspMessage, DspPayload, MessageMessage, QuitMessage},
};
use anyhow::{Context, anyhow};
use log::*;
use tokio::sync::Mutex;
use tui_logger::*;

pub struct AppState {
    client_reader: Option<DspReader>,
    client_writer: Arc<Mutex<Option<DspWriter>>>,
    pub client_config: DspClientConfig,
    app_event_rx: Option<Receiver<AppEvent>>,
    app_event_tx: Sender<AppEvent>,
    pub mode: AppMode,
    pub tab_names: Vec<&'static str>,
    pub log_state: TuiWidgetState,
    pub selected_tab: usize,
    pub active_message: String,
}

impl AppState {
    pub fn new(
        client_reader: DspReader,
        client_writer: DspWriter,
        client_config: DspClientConfig,
    ) -> AppState {
        let (app_event_tx, app_event_rx) = mpsc::channel::<AppEvent>();

        let log_state = TuiWidgetState::new().set_default_display_level(LevelFilter::Trace);

        // Adding this line had provoked the bug as described in issue #69
        let tab_names = vec!["Message", "Quit"];
        AppState {
            client_reader: Some(client_reader),
            client_writer: Arc::new(Mutex::new(Some(client_writer))),
            client_config,
            app_event_tx,
            app_event_rx: Some(app_event_rx),
            mode: AppMode::Run,
            log_state,
            tab_names,
            selected_tab: 0,
            active_message: String::from(""),
        }
    }

    fn send_ui_fatal(tx: Sender<AppEvent>, error: String) {
        loop {
            if let None = tx.send(AppEvent::FatalError(error.clone())).err() {
                return;
            }
        }
    }

    pub fn start(&mut self) -> anyhow::Result<Receiver<AppEvent>> {
        // Use an mpsc::channel to combine stdin events with app events
        let event_rx = self
            .app_event_rx
            .take()
            .ok_or(anyhow!("App initialized without UI event receiver"))?;
        let event_tx = self.app_event_tx.clone();
        let error_tx = self.app_event_tx.clone();
        let payload_receive_tx = self.app_event_tx.clone();
        let client_reader = self
            .client_reader
            .take()
            .ok_or(anyhow!("App initialized without DSP reader"))?;

        thread::spawn(move || input_thread(event_tx));
        tokio::spawn(async move {
            let task = payload_receive_task(client_reader, payload_receive_tx);
            if let Some(err) = task
                .await
                .with_context(|| format!("Connection closed, you need to restart the client"))
                .err()
            {
                AppState::send_ui_fatal(error_tx, err.to_string());
            };
        });

        Ok(event_rx)
    }

    pub fn rerender(&mut self) {
        thread::sleep(Duration::from_millis(10));
        let _ = self.app_event_tx.send(AppEvent::Rerender());
    }

    pub fn rerender_chat(&mut self) {
        self.log_state.transition(TuiWidgetEvent::EscapeKey);
        self.rerender();
    }

    pub fn react_to_payload(&mut self, payload: DspPayload) {
        let username = payload.username;
        let message = payload.message;
        match message {
            DspMessage::JoinMessage(_) => {
                debug!(target: NS_CHAT, "User '{}' has joined the server", username)
            }
            DspMessage::QuitMessage(_) => {
                debug!(target: NS_CHAT, "User '{}' has left the server", username)
            }
            DspMessage::MessageMessage(m) => info!(target: NS_CHAT, "[{}] {}", username, m.text),
            DspMessage::ChallengeMessage(_) => {
                error!(target: NS_CHAT, "You've received a rate-limiting challenge which is not implemented. It's left unimplemented, you might get disconnected.")
            }
            DspMessage::RescindedMessage(_) => {
                warn!(target: NS_CHAT, "The challenge has been rescinded, you can chat again")
            }
            DspMessage::ResponseMessage(_) => {
                warn!(target: NS_CHAT, "You've received a challenge response, this shouldn't happen. Inform server admin.")
            }
            DspMessage::ErrorMessage(m) => error!(target: NS_CHAT, "Server error: {}", m.text),
        }
        self.rerender_chat();
    }

    fn handle_ui_event(&mut self, event: Event) {
        let selected_tab = self.selected_tab;

        if let Event::Key(key) = event {
            let code = key.code;

            match code.into() {
                // Tab switching
                Key::Char('\t') => self.next_tab(),
                Key::Tab => self.next_tab(),

                // Chat scrolling
                Key::Esc => self.log_state.transition(TuiWidgetEvent::EscapeKey),
                Key::PageUp => self.log_state.transition(TuiWidgetEvent::PrevPageKey),
                Key::PageDown => self.log_state.transition(TuiWidgetEvent::NextPageKey),

                // Message sending
                Key::Char(c) if selected_tab == 0 => self.add_active_message(c),
                Key::Backspace if selected_tab == 0 => self.backspace_active_message(),
                Key::Enter if selected_tab == 0 => self.send_active_message(),

                // Quitting
                Key::Enter if selected_tab == 1 => self.trigger_quit(),

                _ => (),
            }
        }
    }
    pub fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::UiEvent(event) => self.handle_ui_event(event),
            AppEvent::PayloadReceived(payload) => self.react_to_payload(payload),
            AppEvent::PayloadSent((writer, payload)) => match payload.message {
                DspMessage::QuitMessage(_) => self.mode = AppMode::Quit,
                DspMessage::MessageMessage(_) => self.active_message_sent(writer),
                _ => {}
            },
            AppEvent::FatalError(error) => {
                error!(target: NS_APP, "{}", error);
                self.rerender_chat();
            }
            AppEvent::Rerender() => {}
        }
    }

    pub fn next_tab(&mut self) {
        self.selected_tab = (self.selected_tab + 1) % self.tab_names.len();
    }

    pub fn add_active_message(&mut self, c: char) {
        self.active_message.push(c);
    }

    pub fn backspace_active_message(&mut self) {
        self.active_message.pop();
    }

    pub fn trigger_quit(&mut self) {
        self.send_payload(DspPayload {
            username: self.client_config.username.clone(),
            message: DspMessage::QuitMessage(QuitMessage {}),
        });
    }

    fn send_payload(&mut self, payload: DspPayload) {
        let ui_tx = self.app_event_tx.clone();
        let mut client_writer = match self.client_writer.try_lock() {
            Ok(mut maybe_writer) => match maybe_writer.take() {
                Some(writer) => writer,
                None => {
                    error!(target: NS_CHAT, "Failed to send message, another message might already be in-transit");
                    return;
                }
            },
            Err(_) => {
                error!(target: NS_CHAT, "Failed to send message, another message is about to possibly go in-transit");
                return;
            }
        };

        tokio::spawn(async move {
            if let Some(err) = client_writer
                .write(payload.clone())
                .await
                .with_context(|| format!("Failed to send message to server"))
                .err()
            {
                AppState::send_ui_fatal(ui_tx.clone(), err.to_string());
            }
            if let Some(err) = ui_tx
                .send(AppEvent::PayloadSent((client_writer, payload)))
                .with_context(|| format!("Failed to return DspWriter to UI"))
                .err()
            {
                AppState::send_ui_fatal(ui_tx.clone(), err.to_string());
            }
        });
    }

    pub fn send_active_message(&mut self) {
        let username = self.client_config.username.clone();
        let text = self.active_message.clone();
        if text.is_empty() {
            error!(target: NS_CHAT, "Can't send empty message");
            return;
        }
        let message = DspMessage::MessageMessage(MessageMessage { text: text.clone() });
        let payload = DspPayload {
            username: username.clone(),
            message,
        };
        self.send_payload(payload);
    }

    pub fn active_message_sent(&mut self, writer: DspWriter) {
        loop {
            match self.client_writer.try_lock() {
                Ok(mut parking) => {
                    // Got access to mutex, left writer back in app
                    let _ = parking.insert(writer);
                    self.active_message = String::from("");
                    return;
                }
                _ => {
                    // Couldn't get access to mutex, retry later?
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }
            }
        }
    }
}

async fn payload_receive_task(
    mut reader: DspReader,
    tx: mpsc::Sender<AppEvent>,
) -> anyhow::Result<()> {
    loop {
        let payload = reader.read().await?;
        tx.send(AppEvent::PayloadReceived(payload))
            .with_context(|| format!("Failed to send received DSP message to UI"))?;
    }
}
