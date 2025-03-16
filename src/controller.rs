use anyhow::Result;

use crate::{client::DspClient, config::Config};

pub struct ControllerState {

}

pub struct Controller {
    client: DspClient,
    state: ControllerState,
}

impl Controller {
    pub async fn run_loop(self) -> Result<()> {
        loop {
            return Ok(())
        }
        // // Join
        // let join_payload = DspPayload { username: config.username.clone(), message: DspMessage::JoinMessage(JoinMessage {}) };
        // info!("Joining... {:?}", join_payload);
        // let _ = client.send_payload(join_payload).await?;
        // info!("Joined!");

        // // See if anyone posts anything else
        // info!("Waiting for another message ...");
        // let another_message = client.read_next_payload().await;
        // info!("Another message! {:?}", another_message);
    }

    pub async fn init(config: Config) -> Result<Controller> {
        // Connect to server
        let mut client: DspClient = DspClient::spawn(&config.client).await?;

        // Set initial state
        let state = ControllerState {};

        // Return controller
        Ok(Controller { client, state })
    }
}