#[macro_use]
extern crate clap;

pub mod grpc_stream;
pub mod substrate_chain;
pub mod solana_chain;
pub mod command;

use lazy_static::lazy_static;
use grpc_stream::stream_mod::{streamout_server::StreamoutServer, ChainType};
use command::{Config, ChainConfig};

lazy_static! {
    // Load default config
    pub static ref CONFIG: Config = Config{
        chains: [
            (ChainType::Substrate,ChainConfig{
                url: "".to_string(),
                ws: "".to_string(),
            }),
            (ChainType::Solana,ChainConfig{
                url: "https://api.mainnet-beta.solana.com".to_string(),
                ws: "wss://api.mainnet-beta.solana.com".to_string(),
            }),
        ].iter().cloned().collect(),
        url: "127.0.0.1:50051".to_string(),
    };
}



