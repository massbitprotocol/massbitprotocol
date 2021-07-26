mod mapping;
mod model;

use massbit_chain_substrate::data_type as types;
use plugin::core::{self, PluginRegistrar};
use store::Store;
use std::error::Error;

#[doc(hidden)]
#[no_mangle]
pub static mut STORE: Option<&dyn Store> = None;

plugin::export_plugin!(register);

#[allow(dead_code, improper_ctypes_definitions)]
extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_substrate_block_handler("handle_block", Box::new(SubstrateBlockHandler));
    registrar.register_substrate_extrinsic_handler("handle_extrinsic", Box::new(SubstrateExtrinsicHandler));
    registrar.register_substrate_event_handler("handle_event", Box::new(SubstrateEventHandler));
    registrar.register_solana_block_handler("handle_block", Box::new(SolanaBlockHandler));
    registrar.register_solana_transaction_handler("handle_transaction", Box::new(SolanaTransactionHandler));
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubstrateBlockHandler;

impl core::SubstrateBlockHandler for SubstrateBlockHandler {
    fn handle_block(&self, block: &types::SubstrateBlock) -> Result<(), Box<dyn Error>> {
        mapping::handle_block(block)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubstrateExtrinsicHandler;

impl core::SubstrateExtrinsicHandler for SubstrateExtrinsicHandler {
    fn handle_extrinsic(&self, extrinsic: &types::SubstrateExtrinsic) -> Result<(), Box<dyn Error>> {
        mapping::handle_extrinsic(extrinsic)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubstrateEventHandler;

impl core::SubstrateEventHandler for SubstrateEventHandler {
    fn handle_event(&self, event: &types::SubstrateEventRecord) -> Result<(), Box<dyn Error>> {
        mapping::handle_event(event)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SolanaBlockHandler;

impl core::SolanaBlockHandler for SolanaBlockHandler {
    fn handle_block(&self, block: &types::SolanaBlock) -> Result<(), Box<dyn Error>> {
        mapping::handle_block(block)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SolanaTransactionHandler;

impl core::SolanaTransactionHandler for SolanaTransactionHandler {
    fn handle_transaction(&self, transaction: &types::SolanaTransaction) -> Result<(), Box<dyn Error>> {
        mapping::handle_transaction(transaction)
    }
}