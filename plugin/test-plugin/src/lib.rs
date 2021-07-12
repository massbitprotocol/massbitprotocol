mod mapping;
mod models;

use massbit_chain_substrate::data_type::SubstrateBlock;
use plugin::core::{BlockHandler as BlockHandlerTrait, PluginRegistrar};
use std::error::Error;
use store::Store;

#[doc(hidden)]
#[no_mangle]
pub static mut STORE: Option<&dyn Store> = None;

plugin::export_plugin!(register);

#[allow(dead_code, improper_ctypes_definitions)]
extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_block_handler("handleBlock", Box::new(BlockHandler));
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockHandler;

impl BlockHandlerTrait for BlockHandler {
    fn handle_substrate_block(&self, block: &SubstrateBlock) -> Result<(), Box<dyn Error>> {
        mapping::handle_block(block)
    }
}
