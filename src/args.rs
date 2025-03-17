use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Name of the person to greet
    #[arg(short, long, default_value_t = String::from("185.216.203.250:1337"))]
    pub server_address: String,

    #[arg(short, long)]
    pub username: String,

    #[arg(short, long)]
    pub log_file: Option<String>,
}
