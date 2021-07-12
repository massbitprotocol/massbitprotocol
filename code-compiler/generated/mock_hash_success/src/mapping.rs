use super::models::NewBlock;
use super::schema::blocks;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use massbit_chain_substrate::data_type::SubstrateBlock;
use plugin::core::{BlockHandler, InvocationError};
use std::env;
use index_store::core::IndexStore;

#[derive(Debug, Clone, PartialEq)]
pub struct BlockIndexer;

impl BlockHandler for BlockIndexer {
    fn handle_block(&self, store: &IndexStore, substrate_block: &SubstrateBlock) -> Result<(), InvocationError> {
        println!("[.SO File] triggered!");

        let number = substrate_block.header.number as i64;
        let new_block = NewBlock { number };

        store.save(blocks::table, new_block);
        Ok(())
    }
}
