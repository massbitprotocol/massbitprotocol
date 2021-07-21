mod mapping;
mod models;

use index_store::core::Store;
use massbit_chain_substrate::data_type::SubstrateEventRecord;
use massbit_chain_substrate::data_type::SubstrateUncheckedExtrinsic;
use massbit_chain_substrate::data_type::SubstrateBlock;
use plugin::core::{PluginRegistrar, SubstrateEventHandler as SubstrateEventHandlerTrait, SubstrateExtrinsicHandler as SubstrateExtrinsicHandlerTrait, SubstrateBlockHandler as SubstrateBlockHandlerTrait};

#[doc(hidden)]
#[no_mangle]
pub static mut STORE: Option<&mut dyn Store> = None;

plugin::export_plugin!(register);

#[allow(dead_code, improper_ctypes_definitions)]
extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_substrate_event_handler(Box::new(SubstrateEventHandler));
    registrar.register_substrate_extrinsic_handler(Box::new(SubstrateExtrinsicHandler));
    registrar.register_substrate_block_handler(Box::new(SubstrateBlockHandler));
}

// Event Handler
#[derive(Debug, Clone, PartialEq)]
pub struct SubstrateEventHandler;
impl SubstrateEventHandlerTrait for SubstrateEventHandler {
    fn handle_event(&self, event: &SubstrateEventRecord) -> Result<(), Box<dyn std::error::Error>> {
        mapping::handle_event(event)
    }
}

// Extrinsic Handler
#[derive(Debug, Clone, PartialEq)]
pub struct SubstrateExtrinsicHandler;
impl SubstrateExtrinsicHandlerTrait for SubstrateExtrinsicHandler {
    fn handle_extrinsic(&self, extrinsic: &SubstrateUncheckedExtrinsic) -> Result<(), Box<dyn std::error::Error>> {
        mapping::handle_extrinsic(extrinsic)
    }
}

// Block Handler
#[derive(Debug, Clone, PartialEq)]
pub struct SubstrateBlockHandler;
impl SubstrateBlockHandlerTrait for SubstrateBlockHandler {
    fn handle_block(&self, block: &SubstrateBlock) -> Result<(), Box<dyn std::error::Error>> {
        mapping::handle_block(block)
    }
}
