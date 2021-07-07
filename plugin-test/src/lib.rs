mod mapping;
mod models;

use plugin::core::PluginRegistrar;
use store::Store;

#[doc(hidden)]
#[no_mangle]
pub static mut STORE: Option<&mut dyn Store> = None;

plugin::export_plugin!(register);
#[allow(dead_code)]
extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_block_handler("test", Box::new(mapping::Indexer));
}
