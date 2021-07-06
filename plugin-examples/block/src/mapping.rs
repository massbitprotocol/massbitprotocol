use super::models::NewBlock;
use super::schema::blocks;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use massbit_chain_substrate::data_type::SubstrateBlock;
use plugin::core::{BlockHandler, InvocationError};
use std::env;
use crate::models::{Store};
// use crate::models::{Config, Store};

#[derive(Debug, Clone, PartialEq)]
pub struct BlockIndexer;

impl BlockHandler for BlockIndexer {
    // fn handle_block(&self, store: Store, substrate_block: &SubstrateBlock) -> Result<(), InvocationError> {
    fn handle_block(&self, connection_string: &String, substrate_block: &SubstrateBlock) -> Result<(), InvocationError> {
        // Should be passed from index manager
       //  let config = Config {
       //      connection_string: String::from(connection_string),
       //      table_name: String::from("abc"),
       // };

        let number = substrate_block.header.number as i64;
        let new_block = NewBlock { number };
        println!("[Mapping] Inserting to database ......");
        // new_block.save(&config);
        new_block.save();
        Ok(())
    }
}
