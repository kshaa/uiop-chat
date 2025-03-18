use crate::{client::DspWriter, protocol::DspPayload};
pub use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode as Key},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

pub enum AppEvent {
    UiEvent(Event),
    PayloadReceived(DspPayload),
    PayloadSent((DspWriter, DspPayload)),
    FatalError(String),
    Rerender(),
}
