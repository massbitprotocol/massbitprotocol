mod mapping;
mod models;

use index_store::core::Store;
use massbit_chain_substrate::data_type::SubstrateUncheckedExtrinsic;
use plugin::core::{PluginRegistrar, SubstrateExtrinsicHandler as SubstrateExtrinsicHandlerTrait};

#[doc(hidden)]
#[no_mangle]
pub static mut STORE: Option<&dyn Store> = None;

plugin::export_plugin!(register);

#[allow(dead_code, improper_ctypes_definitions)]
extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_substrate_extrinsic_handler(Box::new(SubstrateExtrinsicHandler));
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubstrateExtrinsicHandler;

impl SubstrateExtrinsicHandlerTrait for SubstrateExtrinsicHandler {
    fn handle_extrinsic(&self, extrinsic: &SubstrateUncheckedExtrinsic) -> Result<(), Box<dyn std::error::Error>> {
        mapping::handle_extrinsic(extrinsic)
    }
}
