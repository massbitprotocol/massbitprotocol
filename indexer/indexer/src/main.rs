#[macro_use]
extern crate common;
#[macro_use]
extern crate diesel;

use diesel::prelude::*;

fn main() {
    let database_url = "postgres://postgres:postgres@localhost".to_string();
    let connection = PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));
    let block = create_block(&connection);
}

#[derive(Table, Insertable, Queryable)]
#[table_name = "blocks"]
pub struct Block {
    #[column_type = "BigInt"]
    pub id: i64,
}

pub fn create_block(conn: &PgConnection) -> Block {
    let block = Block { id: 1 };
    diesel::insert_into(blocks::table)
        .values(&block)
        .get_result(conn)
        .expect("Error saving new post")
}
