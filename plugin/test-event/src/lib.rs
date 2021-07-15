mod mapping;
mod models;

use index_store::core::Store;
use massbit_chain_substrate::data_type::SubstrateEventRecord;
use plugin::core::{PluginRegistrar, SubstrateEventHandler as SubstrateEventHandlerTrait};

#[doc(hidden)]
#[no_mangle]
pub static mut STORE: Option<&dyn Store> = None;

plugin::export_plugin!(register);

#[allow(dead_code, improper_ctypes_definitions)]
extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_substrate_event_handler(Box::new(SubstrateEventHandler));
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubstrateEventHandler;

impl SubstrateEventHandlerTrait for SubstrateEventHandler {
    fn handle_event(&self, event: &SubstrateEventRecord) -> Result<(), Box<dyn std::error::Error>> {
        mapping::handle_event(event)
    }
}
