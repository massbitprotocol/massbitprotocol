mod mapping;
mod models;

use index_store::core::Store;
use massbit_chain_solana::data_type::{
    SolanaBlock,
    SolanaTransaction,
    SolanaLogMessages,
};
use plugin::core::{
    PluginRegistrar,
    SolanaBlockHandler as SolanaBlockHandlerTrait,
    SolanaTransactionHandler as SolanaTransactionHandlerTrait,
    SolanaLogMessagesHandler as SolanaLogMessagesHandlerTrait
};

#[doc(hidden)]
#[no_mangle]
pub static mut STORE: Option<&mut dyn Store> = None;

plugin::export_plugin!(register);

#[allow(dead_code, improper_ctypes_definitions)]
extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_solana_block_handler(Box::new(SolanaBlockHandler));
    registrar.register_solana_transaction_handler(Box::new(SolanaTransactionHandler));
    registrar.register_solana_event_handler(Box::new(SolanaLogMessagesHandler));
}

// Event Handler
#[derive(Debug, Clone, PartialEq)]
pub struct SolanaLogMessagesHandler;
impl SolanaLogMessagesHandlerTrait for SolanaLogMessagesHandler {
    fn handle_log_messages(&self, event: &SolanaLogMessages) -> Result<(), Box<dyn std::error::Error>> {
        mapping::handle_log_messages(event)
    }
}

// Extrinsic / Transaction Handler
#[derive(Debug, Clone, PartialEq)]
pub struct SolanaTransactionHandler;
impl SolanaTransactionHandlerTrait for SolanaTransactionHandler {
    fn handle_transaction(&self, transaction: &SolanaTransaction) -> Result<(), Box<dyn std::error::Error>> {
        mapping::handle_transaction(transaction)
    }
}

// Block Handler
#[derive(Debug, Clone, PartialEq)]
pub struct SolanaBlockHandler;
impl SolanaBlockHandlerTrait for SolanaBlockHandler {
    fn handle_block(&self, block: &SolanaBlock) -> Result<(), Box<dyn std::error::Error>> {
        mapping::handle_block(block)
    }
}
