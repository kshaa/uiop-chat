use anyhow::{Context, Result};
use log::LevelFilter;
use tui_logger::set_default_level;

pub const NS_CONN: &'static str = "connection";
pub const NS_CHAT: &'static str = "chat";
pub const NS_APP: &'static str = "app";

pub fn init_logger() -> Result<()> {
    tui_logger::init_logger(LevelFilter::Debug).with_context(|| format!("Failed to init TUI chat logger"))?;
    set_default_level(LevelFilter::Debug);
    Ok(())
}
