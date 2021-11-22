pub const INDEXER_LIB: &str = r#"
pub mod generated;
pub mod mapping;

use massbit_solana_sdk::{
    export_plugin,
    plugin::{handler::SolanaHandler, PluginRegistrar},
    store::Store,
    types::SolanaBlock,
};
use lazy_static::lazy_static;
use solana_client::rpc_client::RpcClient;
use std::env;
use std::error::Error;
use std::sync::Arc;

lazy_static! {
    pub static ref SOLANA_CLIENT: Arc<RpcClient> = Arc::new(RpcClient::new(
        env::var("SOLANA_RPC_URL").unwrap_or(String::from("http://194.163.156.242:8899"))
    ));
}
pub const ADDRESS: &str = "{{address}}";

#[doc(hidden)]
#[no_mangle]
pub static mut STORE: Option<&mut dyn Store> = None;

export_plugin!(register);

#[allow(dead_code, improper_ctypes_definitions)]
extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_solana_handler(Box::new(SolanaHandlerAdapter));
}

#[derive(Debug, Clone, PartialEq)]
pub struct SolanaHandlerAdapter;

impl SolanaHandler for SolanaHandlerAdapter {
    fn handle_block(&self, block: &SolanaBlock) -> Result<(), Box<dyn Error>> {
        mapping::handle_block(block)
    }
}
"#;
