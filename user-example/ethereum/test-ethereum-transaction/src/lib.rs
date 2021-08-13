mod mapping;
mod models;
use massbit_chain_ethereum::data_type as ethereum_types;
use adapter::ethereum::*;
use adapter::core::PluginRegistrar;
use index_store::core::Store;
use std::error::Error;

#[doc(hidden)]
#[no_mangle]
pub static mut STORE: Option<&mut dyn Store> = None;

adapter::export_plugin!(register);

#[allow(dead_code, improper_ctypes_definitions)]
extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_ethereum_handler(Box::new(EthereumHandlerAdapter));
}

#[derive(Debug, Clone, PartialEq)]
pub struct EthereumHandlerAdapter;

impl EthereumHandler for EthereumHandlerAdapter {
}