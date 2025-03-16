use anyhow::{Context, Result};
use log::LevelFilter;

pub const NS_CONN: &'static str = "connection";
pub const NS_CHAT: &'static str = "chat";

pub fn init_logger() -> Result<()> {
    tui_logger::init_logger(LevelFilter::Debug).with_context(|| format!("Failed to init TUI chat logger"))
}
