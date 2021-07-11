mod mapping;
mod models;

use massbit_chain_substrate::data_type::SubstrateBlock;
use plugin::core::{BlockHandler, PluginRegistrar};
use store::Store;

#[doc(hidden)]
#[no_mangle]
pub static mut STORE: Option<&dyn Store> = None;

plugin::export_plugin!(register);

#[allow(dead_code, improper_ctypes_definitions)]
extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_block_handler("test", Box::new(Indexer));
}

#[derive(Debug, Clone, PartialEq)]
pub struct Indexer;

impl BlockHandler for Indexer {
    fn handle_block(&self, block: &SubstrateBlock) -> Result<(), Box<dyn std::error::Error>> {
        mapping::handle_block(block)
    }
}
