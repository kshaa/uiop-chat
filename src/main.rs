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
    // Init logger
    let _ = init_logger()?;

    // Parse CLI args
    let args = Args::try_parse().with_context(|| format!("Invalid CLI arguments passed"))?;

    // Build config
    let config  = Config::from_args(args);

    // Start DSP client
    let mut client = DspClient::spawn(&config.client).await?;

    // Join the server
    client.writer.write(DspPayload { 
        username: config.client.username.clone(), 
        message: uiop_dsp::protocol::DspMessage::JoinMessage(JoinMessage {} )}
    ).await?;
    info!(target: NS_CONN, "Joined chat server");

    // Init chat app
    let app = App::new(client.reader, client.writer, config.client);
    app.start_with_crossterm()?;

    // App closed successfully, quitting
    Ok(())
}
