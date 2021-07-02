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
    fn handle_block(&self, substrate_block: &SubstrateBlock) -> Result<(), InvocationError> {
        let connection = establish_connection();
        let number = substrate_block.header.number as i64;
        let new_block = NewBlock { number };
        println!("Inserting to database..");
        let _ = diesel::insert_into(blocks::table)
            .values(&new_block)
            .execute(&connection);
        Ok(())
    }
}

pub fn establish_connection() -> PgConnection {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}
