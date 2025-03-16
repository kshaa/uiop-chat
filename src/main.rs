use uiop_dsp::config::Config;
use uiop_dsp::controller::Controller;
use uiop_dsp::logger::init_logger;
use uiop_dsp::args::*;
use clap::Parser;
use anyhow::Result;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Init logger
    let _ = init_logger()?;

    // Parse CLI args
    let args = Args::try_parse()?;

    // Build config
    let config  = Config::from_args(args);

    // Init chat controller
    let controller = Controller::init(config).await?;

    // Execute chat controller run-loop
    let _ = controller.run_loop().await?;

    // Control loop closed successfully, quitting
    Ok(())
}
