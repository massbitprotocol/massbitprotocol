use massbit_chain_substrate::data_type::SubstrateBlock;
use store::Store;

pub trait BlockHandler {
    fn handle_block(
        &self,
        store: &mut dyn Store,
        block: &SubstrateBlock,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

#[derive(Copy, Clone)]
pub struct PluginDeclaration {
    pub register: unsafe extern "C" fn(&mut dyn PluginRegistrar),
}

pub trait PluginRegistrar {
    fn register_block_handler(&mut self, name: &str, function: Box<dyn BlockHandler>);
}
