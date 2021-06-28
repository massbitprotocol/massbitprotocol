mod mapping;
mod models;

#[macro_use]
extern crate diesel_derive_table;
#[macro_use]
extern crate diesel;

use diesel::pg::PgConnection;
use plugin_core::PluginRegistrar;

plugin_core::export_plugin!(register);

#[doc(hidden)]
#[no_mangle]
pub static mut CONN: Option<PgConnection> = None;

extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_block_handler("handle_block", Box::new(mapping::BlockIndexer));
    registrar.register_extrinsic_handler("handle_extrinsic", Box::new(mapping::ExtrinsicIndexer));
}
