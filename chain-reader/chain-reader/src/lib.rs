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
use grpc_stream::stream_mod::ChainType;
use lazy_static::lazy_static;

lazy_static! {
    // Load default config
    pub static ref CONFIG: Config = Config{
        chains: [
            // ChainConfig{
            //     url: "".to_string(),
            //     ws: "".to_string(),
            //     start_block: None,
            //     chain_type: ChainType::Substrate,
            //     network: "mainnet".to_string()
            // },
            // ChainConfig{
            //     ws: "ws://api.mainnet-beta.solana.com".to_string(),
            //     // url: "https://mainnet-beta-solana.massbit.io".to_string(),
            //     //url: "https://api.mainnet-beta.solana.com".to_string(),
            //     url: "https://solana-api.projectserum.com".to_string(),
            //
            //     start_block: None,
            //     chain_type: ChainType::Solana,
            //     network: "mainnet".to_string()
            // },
            ChainConfig{
                ws: "wss://rpc-mainnet.matic.network".to_string(),
                //url: "https://rpc-mainnet.matic.network".to_string(),
                url: "https://polygon-rpc.com/".to_string(),
                //start_block: Some(18403764),
                //start_block: Some(18404360),
                //start_block: Some(18504360),
                start_block: Some(18551330),
                chain_type: ChainType::Ethereum,
                network: "matic".to_string()
            },
            ChainConfig{
                ws: "wss://bsc-ws-node.nariox.org:443".to_string(),
                url: "https://bsc-dataseed.binance.org".to_string(),
                start_block: None,
                chain_type: ChainType::Ethereum,
                network: "bsc".to_string()
            },
            ChainConfig{
                ws: "wss://main-light.eth.linkpool.io/ws".to_string(),
                url: "https://main-light.eth.linkpool.io".to_string(),
                start_block: None,
                chain_type: ChainType::Ethereum,
                network: "ethereum".to_string()
            },
        ].iter().cloned().collect(),
        url: "0.0.0.0:50051".to_string(),
    };
}
