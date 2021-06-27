#[macro_use]
extern crate diesel_derive_table;
#[macro_use]
extern crate diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use plugins_core::{Function, InvocationError, PluginRegistrar};
use types::SubstrateBlock;

plugins_core::export_plugin!(register);

#[doc(hidden)]
#[no_mangle]
pub static mut CONN: Option<PgConnection> = None;

extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_function("handle_block", Box::new(BlockIndexer));
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockIndexer;

impl Function for BlockIndexer {
    fn handle_block(&self, substrate_block: &SubstrateBlock) -> Result<(), InvocationError> {
        let block = Block {
            id: substrate_block.idx,
        };
        let mut tmp_block = Block { id: 2 };
        unsafe {
            let conn = CONN.as_ref().unwrap();
            tmp_block = diesel::insert_into(blocks::table)
                .values(&block)
                .get_result(conn)
                .expect("Error saving new post");
        }
        Ok(())
    }
}

#[derive(Table, Insertable, Queryable)]
#[table_name = "blocks"]
pub struct Block {
    #[column_type = "BigInt"]
    pub id: i64,
}
