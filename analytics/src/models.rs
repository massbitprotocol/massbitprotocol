use crate::schema::*;
use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel::types::{Int8, Bytea};
use diesel::Expression;

//https://kotiri.com/2018/01/31/postgresql-diesel-rust-types.html
#[derive(Insertable, Queryable)]
#[table_name = "matic_block"]
pub struct MaticBlock {
    pub block_hash: String,
    pub block_height: i64,
    pub transaction_number: i64,
    pub timestamp: i64,
    pub validated_by: Option<String>,
    pub reward: Option<BigDecimal>,
    pub difficulty: Option<i64>,
    pub total_difficulty: Option<i64>,
    pub size: Option<i64>,
    pub gas: Option<BigDecimal>,
    pub gas_limit: Option<BigDecimal>,
    pub extra_data: Option<Vec<u8>>,
}

#[derive(Insertable)]
#[table_name = "matic_transaction"]
pub struct MaticTransaction {
    pub transaction_hash: String,
    pub block_hash: Option<String>,
    pub block_number: Option<i64>,
    pub nonce: Option<BigDecimal>,
    pub sender: String,
    pub receiver: Option<String>,
    pub value: BigDecimal,
    pub gas_limit: BigDecimal,
    pub gas_price: BigDecimal,
    pub timestamp: i64
}
