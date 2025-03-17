use crate::args::Args;

pub struct DspLogConfig {
    pub log_file: Option<String>,
}

pub struct DspClientConfig {
    pub server_address: String,
    pub username: String,
}

pub struct Config {
    pub client: DspClientConfig,
    pub log: DspLogConfig,
}

impl Config {
    pub fn from_args(args: Args) -> Config {
        let server_address = args.server_address;
        let username = args.username;
        let client = DspClientConfig {
            server_address,
            username,
        };

        let log_file = args.log_file;
        let log = DspLogConfig {
            log_file,
        };

        Config {
            client,
            log,
        }
    }
}


