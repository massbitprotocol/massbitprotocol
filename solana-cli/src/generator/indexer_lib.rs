pub const indexer_lib: &str = r#"
pub mod mapping;
//pub mod models;
pub mod generated;

use adapter::core::PluginRegistrar;
use adapter::solana::*;
use index_store::core::Store;
pub use index_store::{Attribute, Entity, EntityFilter, EntityOrder, EntityRange, Value};
pub use index_store::{EntityValue, FromEntity, FromValueTrait, ToMap, ValueFrom};
use lazy_static::lazy_static;
use massbit_chain_solana::data_type as solana_types;
use solana_client::rpc_client::RpcClient;
use std::env;
use std::error::Error;
use std::sync::Arc;

lazy_static! {
    pub static ref SOLANA_CLIENT: Arc<RpcClient> = Arc::new(RpcClient::new(
        env::var("SOLANA_RPC_URL").unwrap_or(String::from("http://194.163.156.242:8899"))
    ));
}

#[doc(hidden)]
#[no_mangle]
pub static mut STORE: Option<&mut dyn Store> = None;

adapter::export_plugin!(register);

#[allow(dead_code, improper_ctypes_definitions)]
extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_solana_handler(Box::new(SolanaHandlerAdapter));
}

#[derive(Debug, Clone, PartialEq)]
pub struct SolanaHandlerAdapter;

impl SolanaHandler for SolanaHandlerAdapter {
    fn handle_block(&self, block: &solana_types::SolanaBlock) -> Result<(), Box<dyn Error>> {
        mapping::handle_block(block)
    }
}
"#;
