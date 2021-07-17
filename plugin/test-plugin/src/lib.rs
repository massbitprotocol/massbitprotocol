mod mapping;
mod models;

use index_store::core::Store;
use massbit_chain_substrate::data_type::SubstrateBlock;
use plugin::core::{PluginRegistrar, SubstrateBlockHandler as SubstrateBlockHandlerTrait};

#[doc(hidden)]
#[no_mangle]
pub static mut STORE: Option<&dyn Store> = None;

plugin::export_plugin!(register);

#[allow(dead_code, improper_ctypes_definitions)]
extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_substrate_block_handler(Box::new(SubstrateBlockHandler));
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubstrateBlockHandler;
impl SubstrateBlockHandlerTrait for SubstrateBlockHandler {
    fn handle_block(&self, block: &SubstrateBlock) -> Result<(), Box<dyn std::error::Error>> {
        mapping::handle_block(block)
    }
}
