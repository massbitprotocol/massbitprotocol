#[macro_use]
extern crate diesel;

pub mod mapping;
pub mod models;
pub mod schema;

use plugin::core::PluginRegistrar;

plugin::export_plugin!(register);

#[allow(dead_code)]
extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_block_handler("handle_block", Box::new(mapping::BlockIndexer));
}
