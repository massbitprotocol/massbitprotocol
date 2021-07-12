use massbit_chain_substrate::data_type::SubstrateBlock;
use std::error::Error;

pub trait BlockHandler {
    fn handle_substrate_block(&self, block: &SubstrateBlock) -> Result<(), Box<dyn Error>>;
}

#[derive(Copy, Clone)]
pub struct PluginDeclaration {
    pub register: unsafe extern "C" fn(&mut dyn PluginRegistrar),
}

pub trait PluginRegistrar {
    fn register_block_handler(&mut self, name: &str, handler: Box<dyn BlockHandler>);
}
