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
            (ChainType::Substrate,ChainConfig{
                url: "".to_string(),
                ws: "".to_string(),
                start_block: None,
            }),
            (ChainType::Solana,ChainConfig{
                ws: "wss://api.mainnet-beta.solana.com".to_string(),
                url: "https://api.mainnet-beta.solana.com".to_string(),
                // url: "https://mainnet-beta-solana.massbit.io".to_string(),
                start_block: None,
            }),
            (ChainType::Ethereum,ChainConfig{
                // ws: "wss://main-light.eth.linkpool.io/ws".to_string(),
                // url: "https://main-light.eth.linkpool.io".to_string(),
                // ws: "wss://bsc-ws-node.nariox.org:443".to_string(),
                // url: "https://bsc-dataseed.binance.org".to_string(),
                ws: "wss://rpc-mainnet.matic.network".to_string(),
                url: "https://polygon-rpc.com/".to_string(),
                start_block: None,  // (9-3-2021) Quickswap https://polygonscan.com/txs?a=0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32&p=37
            }),
        ].iter().cloned().collect(),
        url: "0.0.0.0:50051".to_string(),
    };
}
