use log::info;
use uiop_dsp::protocol::JoinMessage;
use uiop_dsp::{app::App, protocol::DspPayload};
use uiop_dsp::client::DspClient;
use uiop_dsp::config::Config;
use uiop_dsp::logger::{init_logger, NS_CONN};
use uiop_dsp::args::*;
use clap::Parser;
use anyhow::{Context, Result};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse CLI args
    let args = Args::try_parse().with_context(|| format!("Invalid CLI arguments passed"))?;

    // Build config
    let config  = Config::from_args(args);

    // Init logger
    let _ = init_logger(&config.log)?;

    // Start DSP client
    let client = DspClient::start(&config.client).await?;

    // Init chat app
    let app = App::new(client.reader, client.writer, config.client);
    app.start_with_crossterm()?;

    // App closed successfully, quitting
    Ok(())
}
