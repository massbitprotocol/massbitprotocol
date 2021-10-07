#[macro_use]
extern crate clap;

pub mod command;
pub mod ethereum_chain;
pub mod grpc_stream;
pub mod solana_chain;
pub mod substrate_chain;
pub mod transport;
pub use self::transport::Transport;

use command::{ChainConfig, Config};
use lazy_static::lazy_static;
use massbit::firehose::bstream::ChainType;
use std::env;

lazy_static! {
    // Load default config
    static ref SOLANA_WS: String = env::var("SOLANA_WS").unwrap_or(String::from("ws://api.mainnet-beta.solana.com"));
    static ref SOLANA_URL: String = env::var("SOLANA_URL").unwrap_or(String::from("https://solana-api.projectserum.com"));
    //static ref SOLANA_URL: String = env::var("SOLANA_URL").unwrap_or(String::from("http://194.163.156.242:8899"));
    static ref POLYGON_WS: String = env::var("POLYGON_WS").unwrap_or(String::from("wss://rpc-mainnet.matic.network"));
    static ref POLYGON_URL: String = env::var("POLYGON_URL").unwrap_or(String::from("https://polygon-rpc.com"));
    static ref BSC_WS: String = env::var("BSC_WS").unwrap_or(String::from("wss://bsc-ws-node.nariox.org:443"));
    static ref BSC_URL: String = env::var("BSC_URL").unwrap_or(String::from("https://bsc-dataseed.binance.org"));
    static ref ETHEREUM_WS: String = env::var("ETHEREUM_WS").unwrap_or(String::from("wss://main-light.eth.linkpool.io/ws"));
    static ref ETHEREUM_URL: String = env::var("ETHEREUM_URL").unwrap_or(String::from("https://main-light.eth.linkpool.io"));
    static ref HAMONY_WS: String = env::var("HAMONY_WS").unwrap_or(String::from(""));
    static ref HAMONY_URL: String = env::var("HAMONY_WS").unwrap_or(String::from("https://a.api.s0.t.hmny.io/"));
    pub static ref CONFIG: Config = Config{
        chains: [
            //  ChainConfig{
            //     url: "".to_string(),
            //     ws: "".to_string(),
            //     start_block: None,
            //     chain_type: ChainType::Substrate,
            //     network: "mainnet".to_string(),
            //     supports_eip_1898: true,
            // },
            ChainConfig{
                ws: SOLANA_WS.to_string(),
                url: SOLANA_URL.to_string(),
                start_block: None,
                chain_type: ChainType::Solana,
                network: "mainnet".to_string(),
                supports_eip_1898: true,
            },
            ChainConfig{
                ws: POLYGON_WS.to_string(),
                url: POLYGON_URL.to_string(),
                start_block: Some(18403764),
                chain_type: ChainType::Ethereum,
                network: "matic".to_string(),
                supports_eip_1898: true,
            },
            ChainConfig{
                ws: BSC_WS.to_string(),
                url: BSC_URL.to_string(),
                start_block: None,
                chain_type: ChainType::Ethereum,
                network: "bsc".to_string(),
                supports_eip_1898: true,
            },
            ChainConfig{
                ws: ETHEREUM_WS.to_string(),
                url: ETHEREUM_URL.to_string(),
                start_block: None,
                chain_type: ChainType::Ethereum,
                network: "ethereum".to_string(),
                supports_eip_1898: true,
            },
            ChainConfig{
                ws: HAMONY_WS.to_string(),
                url: HAMONY_URL.to_string(),
                start_block: None,
                chain_type: ChainType::Ethereum,
                network: "mainnet".to_string(),
                supports_eip_1898: false,
            },
        ].iter().cloned().collect(),
        url: "0.0.0.0:50051".to_string(),
    };
}
