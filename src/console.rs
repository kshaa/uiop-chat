use anyhow::Result;
use dialoguer::Input;
use crate::config::DspClientConfig;


pub async fn wait_for_message(config: &DspClientConfig) -> Result<String> {
    let message: String = Input::new()
        .with_prompt(format!("[{}]", config.username))
        .interact_text()
        .unwrap();

    Ok(message)
}
