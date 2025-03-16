use crate::args::Args;


pub struct DspClientConfig {
    pub server_address: String,
    pub username: String,
}

pub struct Config {
    pub client: DspClientConfig
}

impl Config {
    pub fn from_args(args: Args) -> Config {
        let server_address = args.server_address;
        let username = args.username;
        let client = DspClientConfig {
            server_address,
            username,
        };

        Config {
            client,
        }
    }
}


