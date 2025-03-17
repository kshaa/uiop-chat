use anyhow::{Context, Result};
use log::LevelFilter;
use tui_logger::{set_default_level, set_log_file, TuiLoggerFile, TuiLoggerLevelOutput};

use crate::config::DspLogConfig;

pub const NS_CONN: &'static str = "connection";
pub const NS_CHAT: &'static str = "chat";
pub const NS_APP: &'static str = "app";

pub fn init_logger(log_config: &DspLogConfig) -> Result<()> {
    tui_logger::init_logger(LevelFilter::Debug).with_context(|| format!("Failed to init TUI chat logger"))?;
    set_default_level(LevelFilter::Debug);
    if let Some(log_file) = &log_config.log_file {
        let file_opts = TuiLoggerFile::new(&log_file)
            .output_separator(' ')
            .output_timestamp(Some("%Y-%m-%dT%H:%M:%S".to_string()))
            .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
            .output_target(true)
            .output_file(false)
            .output_line(false);
        set_log_file(file_opts);
    }
    Ok(())
}
