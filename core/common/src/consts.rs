use super::prelude::lazy_static::lazy_static;
use std::env;
lazy_static! {
    pub static ref SOLANA_WS: String = r#"wss://api.mainnet-beta.solana.com"#.to_string();
    pub static ref SOLANA_URL: String = r#"https://api.mainnet-beta.solana.com"#.to_string();
    pub static ref ETHEREUM_WS: String = r#"wss://rpc-mainnet.matic.network"#.to_string();
    pub static ref ETHEREUM_URL: String = r#"https://rpc-mainnet.matic.network"#.to_string();
    pub static ref ETHEREUM_USE_WS: bool = false;
    pub static ref HASURA_URL: String =
        env::var("HASURA_URL").unwrap_or(String::from("http://localhost:8080/v1/query"));
}
