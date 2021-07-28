mod mapping;
mod models;

use massbit_chain_solana::data_type as types;
use plugin::core::{self, PluginRegistrar};
use index_store::core::Store;
use std::error::Error;

#[doc(hidden)]
#[no_mangle]
pub static mut STORE: Option<&mut dyn Store> = None;

plugin::export_plugin!(register);

#[allow(dead_code, improper_ctypes_definitions)]
extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_solana_transaction_handler(Box::new(SolanaTransactionHandler));
}

#[derive(Debug, Clone, PartialEq)]
pub struct SolanaTransactionHandler;

impl core::SolanaTransactionHandler for SolanaTransactionHandler {
    fn handle_transaction(&self, transaction: &types::SolanaTransaction) -> Result<(), Box<dyn Error>> {
        mapping::handle_transaction(transaction)
    }
}