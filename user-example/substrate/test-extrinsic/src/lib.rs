mod mapping;
mod models;

use massbit_chain_substrate::data_type as substrate_types;
use massbit_chain_solana::data_type as solana_types;
use plugin::core::{self, PluginRegistrar};
use index_store::core::Store;
use std::error::Error;

#[doc(hidden)]
#[no_mangle]
pub static mut STORE: Option<&mut dyn Store> = None;

plugin::export_plugin!(register);

#[allow(dead_code, improper_ctypes_definitions)]
extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_substrate_extrinsic_handler(Box::new(SubstrateExtrinsicHandler));
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubstrateExtrinsicHandler;

impl core::SubstrateExtrinsicHandler for SubstrateExtrinsicHandler {
    fn handle_extrinsic(&self, extrinsic: &substrate_types::SubstrateUncheckedExtrinsic) -> Result<(), Box<dyn Error>> {
        mapping::handle_extrinsic(extrinsic)
    }
}
