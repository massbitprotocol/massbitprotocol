pub mod adapter;
pub mod chain;
pub mod data_source;
pub mod manifest;
pub mod trigger;
pub mod types;

pub use chain::Chain;
pub use manifest::SolanaIndexerManifest;

use core::array::IntoIter;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::ops::Deref;
use types::ChainConfig;

const TRANSACTION_BATCH_SIZE: usize = 100;
// The max value is 1000
const LIMIT_FILTER_RESULT: usize = 1000;

lazy_static! {
    // Load default config
    pub static ref SOLANA_NETWORKS: HashMap<String, ChainConfig> = HashMap::<String, ChainConfig>::from_iter(IntoIter::new([
        ("mainnet".to_string(), ChainConfig
                {
                    ws: "ws://api.mainnet-beta.solana.com".to_string(),
                    url: "https://api.mainnet-beta.solana.com".to_string(),
                    network: "mainnet".to_string(),
                    supports_eip_1898: true,
                }
        ),
        ("projectserum".to_string(), ChainConfig
                {
                    ws: "ws://solana-api.projectserum.com".to_string(),
                    url: "https://solana-api.projectserum.com".to_string(),
                    network: "projectserum".to_string(),
                    supports_eip_1898: true,
                }
        ),
        ("massbit2".to_string(), ChainConfig
                {
                    ws: "ws://194.163.156.242:8899".to_string(),
                    url: "http://194.163.156.242:8899".to_string(),
                    network: "massbit2".to_string(),
                    supports_eip_1898: true,
                }
        ),
        ("massbit3".to_string(), ChainConfig
                {
                    ws: "ws://194.163.186.82:8899".to_string(),
                    url: "http://194.163.186.82:8899".to_string(),
                    network: "massbit3".to_string(),
                    supports_eip_1898: true,
                }
        )
    ]));
}
