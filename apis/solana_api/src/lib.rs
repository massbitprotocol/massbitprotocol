#[macro_use]
extern crate diesel;
pub mod block_api;
pub mod orm;
pub mod rpc_handler;
pub mod transaction_api;
use lazy_static::lazy_static;
use solana_client::rpc_client::RpcClient;
use std::env;
use std::sync::Arc;

lazy_static! {
    pub static ref COMPONENT_NAME: String = String::from("[SolanaApi]");
    pub static ref CONNECTION_POOL_SIZE: u32 = env::var("CONNECTION_POOL_SIZE")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(20);
    pub static ref API_ENDPOINT: String =
        env::var("API_ENDPOINT").unwrap_or(String::from("0.0.0.0:9090"));
    pub static ref SOLANA_CLIENT: Arc<RpcClient> = Arc::new(RpcClient::new(
        env::var("SOLANA_RPC_URL").unwrap_or(String::from("http://194.163.156.242:8899"))
    ));
    pub static ref DATABASE_URL: String = env::var("DATABASE_URL").unwrap_or(String::from(
        "postgres://graph-node:let-me-in@localhost/analytic"
    ));
}
