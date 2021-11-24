#[macro_use]
extern crate clap;

pub mod command;
pub mod grpc_stream;
pub mod solana_chain;

use command::{ChainConfig, Config};
use lazy_static::lazy_static;
use massbit::firehose::bstream::ChainType;
use std::env;

lazy_static! {
    // Load default config
    static ref SOLANA_WS: String = env::var("SOLANA_WS").unwrap_or(String::from("ws://api.mainnet-beta.solana.com"));
    //static ref SOLANA_URL: String = env::var("SOLANA_URL").unwrap_or(String::from("https://solana-api.projectserum.com"));
    //static ref SOLANA_URL: String = env::var("SOLANA_URL").unwrap_or(String::from("https://api.mainnet-beta.solana.com"));
    //static ref SOLANA_URL: String = env::var("SOLANA_URL").unwrap_or(String::from("http://194.163.156.242:8899")); // massbit 2
    static ref SOLANA_URL: String = env::var("SOLANA_URL").unwrap_or(String::from("http://194.163.186.82:8899")); // massbit 3
    pub static ref CONFIG: Config = Config{
        chains: [
            ChainConfig{
                ws: SOLANA_WS.to_string(),
                url: SOLANA_URL.to_string(),
                start_block: None,
                chain_type: ChainType::Solana,
                network: "mainnet".to_string(),
                supports_eip_1898: true,
            },
        ].iter().cloned().collect(),
        url: "0.0.0.0:50051".to_string(),
    };
}
