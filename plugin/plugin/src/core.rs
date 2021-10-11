use massbit_chain_solana::data_type::{SolanaBlock, SolanaLogMessages, SolanaTransaction};

use std::error::Error;

pub trait SolanaBlockHandler {
    fn handle_block(&self, block: &SolanaBlock) -> Result<(), Box<dyn Error>>;
}

pub trait SolanaTransactionHandler {
    fn handle_transaction(&self, extrinsic: &SolanaTransaction) -> Result<(), Box<dyn Error>>;
}

pub trait SolanaLogMessagesHandler {
    fn handle_log_messages(&self, event: &SolanaLogMessages) -> Result<(), Box<dyn Error>>;
}

#[derive(Copy, Clone)]
pub struct PluginDeclaration {
    pub register: unsafe extern "C" fn(&mut dyn PluginRegistrar),
}

pub trait PluginRegistrar {
    fn register_solana_block_handler(&mut self, handler: Box<dyn SolanaBlockHandler>);
    fn register_solana_transaction_handler(&mut self, handler: Box<dyn SolanaTransactionHandler>);
    fn register_solana_event_handler(&mut self, handler: Box<dyn SolanaLogMessagesHandler>);
}
