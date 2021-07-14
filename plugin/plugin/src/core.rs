use massbit_chain_substrate::data_type::{
    SubstrateBlock, SubstrateCheckedExtrinsic, SubstrateEventRecord,
};
use std::error::Error;

pub trait SubstrateBlockHandler {
    fn handle_block(&self, block: &SubstrateBlock) -> Result<(), Box<dyn Error>>;
}

pub trait SubstrateExtrinsicHandler {
    fn handle_extrinsic(&self, extrinsic: &SubstrateCheckedExtrinsic)
        -> Result<(), Box<dyn Error>>;
}

pub trait SubstrateEventHandler {
    fn handle_event(&self, event: &SubstrateEventRecord) -> Result<(), Box<dyn Error>>;
}

#[derive(Copy, Clone)]
pub struct PluginDeclaration {
    pub register: unsafe extern "C" fn(&mut dyn PluginRegistrar),
}

pub trait PluginRegistrar {
    fn register_substrate_block_handler(&mut self, handler: Box<dyn SubstrateBlockHandler>);
    fn register_substrate_extrinsic_handler(&mut self, handler: Box<dyn SubstrateExtrinsicHandler>);
    fn register_substrate_event_handler(&mut self, handler: Box<dyn SubstrateEventHandler>);
}
