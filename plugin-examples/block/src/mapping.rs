use super::models::NewBlock;
use super::schema::blocks;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use massbit_chain_substrate::data_type::SubstrateBlock;
use plugin::core::{BlockHandler, InvocationError};
use std::env;

#[derive(Debug, Clone, PartialEq)]
pub struct BlockIndexer;

impl BlockHandler for BlockIndexer {
    fn handle_block(&self, connection_string: &String, substrate_block: &SubstrateBlock) -> Result<(), InvocationError> {
        let connection = establish_connection(&connection_string);
        let number = substrate_block.header.number as i64;
        let new_block = NewBlock { number };
        println!("[Mapping] Inserting to database..");

        let _ = diesel::insert_into(blocks::table) // Add random hash for the table
            .values(&new_block)
            .execute(&connection);
        Ok(())
    }
}

pub fn establish_connection(connection_string: &String) -> PgConnection {
    PgConnection::establish(connection_string).expect(&format!("Error connecting to {}", connection_string))
}
