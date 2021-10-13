use lazy_static::lazy_static;
use solana_client::rpc_client::RpcClient;
use std::env;
use std::sync::Arc;
lazy_static! {
    static ref SOLANA_CLIENT: Arc<RpcClient> = Arc::new(RpcClient::new(
        env::var("SOLANA_URL").unwrap_or(String::from("http://194.163.156.242:8899"))
    ));
}

pub fn get_owner_account(account: &str) -> Option<String> {
    Some(String::from(""))
}

pub fn get_mint_account(account: &str) -> Option<String> {
    Some(String::from(""))
}
