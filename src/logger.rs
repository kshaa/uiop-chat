use anyhow::{Context, Result};
use env_logger::Env;

pub const NS_CONN: &'static str = "connection";
pub const NS_CHAT: &'static str = "chat";

pub fn init_logger() -> Result<()> {
    let mut builder = env_logger::Builder::from_env(Env::default().default_filter_or("debug"));
    builder.filter_level(log::LevelFilter::Debug);
    builder.try_init().with_context(|| format!("Failed to init logger"))
}
