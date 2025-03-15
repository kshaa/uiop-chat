use log::info;
use uiop_dsp::protocol::*;
use uiop_dsp::client::*;

use anyhow::Result;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Init logger & configs
    let _ = simple_logger::SimpleLogger::new().env().init().unwrap();
    let config = DspClientConfig {
        server_address: "185.216.203.250:1337",
        username: String::from("testname"),
    };

    // Connect to server
    let mut client: DspClient = DspClient::spawn(&config).await?;

    // Receive welcome message
    info!("Waiting for welcome message ...");
    let welcome_message = client.read_next_payload().await?;
    info!("Welcome message received! {:?}", welcome_message);

    // Join
    let join_payload = DspPayload { username: config.username.clone(), message: DspMessage::JoinMessage(JoinMessage {}) };
    info!("Joining... {:?}", join_payload);
    let _ = client.send_payload(join_payload).await?;
    info!("Joined!");

    // See if anyone posts anything else
    info!("Waiting for another message ...");
    let another_message = client.read_next_payload().await;
    info!("Another message! {:?}", another_message);

    Ok(())
}
