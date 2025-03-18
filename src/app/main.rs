use super::{event::AppEvent, mode::AppMode, state::AppState};
use crate::app::crossterm_backend::*;
use crate::{
    client::{DspReader, DspWriter},
    config::DspClientConfig,
};
use ratatui::{prelude::*, widgets::*};
use std::sync::mpsc::{self};
use tui_logger::*;

pub struct App {
    state: AppState,
}

impl App {
    pub fn new(
        client_reader: DspReader,
        client_writer: DspWriter,
        client_config: DspClientConfig,
    ) -> App {
        let state = AppState::new(client_reader, client_writer, client_config);
        App { state }
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

    fn start(mut self, terminal: &mut Terminal<impl Backend>) -> anyhow::Result<()> {
        let event_rx = self.state.start()?;
        self.run(terminal, event_rx)
    }

    /// Main application loop
    fn run(
        &mut self,
        terminal: &mut Terminal<impl Backend>,
        rx: mpsc::Receiver<AppEvent>,
    ) -> anyhow::Result<()> {
        for event in rx {
            self.state.handle_app_event(event);
            if self.state.mode == AppMode::Quit {
                break;
            }
            self.draw(terminal)?;
        }
        Ok(())
    }

    fn draw(&mut self, terminal: &mut Terminal<impl Backend>) -> anyhow::Result<()> {
        terminal.draw(|frame| {
            frame.render_widget(self, frame.area());
        })?;
        Ok(())
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

        Tabs::new(self.state.tab_names.iter().cloned())
            .block(Block::default().title("States").borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .select(self.state.selected_tab)
            .render(tabs_area, buf);

        TuiLoggerWidget::default()
            .block(
                Block::default()
                    .title(format!(
                        "Server chat: {}",
                        self.state.client_config.server_address
                    ))
                    .borders(Borders::ALL),
            )
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
            .state(&self.state.log_state)
            .render(smart_area, buf);

        let prompt_block = Block::new()
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .style(Style::default())
            .title(format!("[{}]", self.state.client_config.username));

        let prompt_block_inner = prompt_block.inner(prompt_area);
        prompt_block.render(prompt_area, buf);

        let style = Style::new().white();
        Text::raw(self.state.active_message.clone())
            .style(style)
            .render(prompt_block_inner, buf);

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
