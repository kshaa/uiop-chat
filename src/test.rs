use log::debug;
use log::error;
use log::info;
use log::warn;
use tokio::task;
use tokio::time;
use uiop_dsp::config::DspClientConfig;
use uiop_dsp::console::wait_for_message;
use uiop_dsp::logger::init_logger;
use uiop_dsp::logger::NS_CHAT;
use anyhow::Result;
use uiop_dsp::logger::NS_CONN;
use std::error::Error;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Init logger
    init_logger()?;

    // Init fake config
    let client_config = DspClientConfig { server_address: String::from("127.0.0.1:8080"), username: String::from("You") };
    
    // Print some realistic-looking chat init logs
    debug!(target: NS_CONN, "Connecting to server...");
    debug!(target: NS_CONN, "Connection established!");
    debug!(target: NS_CONN, "Joining chat room...");
    debug!(target: NS_CHAT, "[server] 'You' has joined");
    warn!(target: NS_CONN, "Received invalid message from server, ignoring");
    info!(target: NS_CHAT, "[Someone] Hi, You!");
    error!(target: NS_CONN, "Server disconnected us, closing");

    // Print something more after 5 seconds
    let _ = task::spawn(async {
        time::sleep(Duration::from_secs(5)).await;
        info!(target: NS_CHAT, "[Someone2] You! Hi!!");
    });

    // Receive a message from user console
    let message = wait_for_message(&client_config).await?;

    // Print message
    info!(target: NS_CHAT, "[You] {}", message);

    // Finish test
    Ok(())
}
