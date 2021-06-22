#[macro_use]
extern crate diesel_derive_table;

pub struct Block {
    pub id: i64,
    pub block_height: i64,
    pub timestamp: i64,
}
