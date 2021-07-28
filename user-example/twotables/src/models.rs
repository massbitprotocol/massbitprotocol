use crate::STORE;
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(Default, Clone, ToMap)]
pub struct Block {
    pub id: String,
    pub block_number: i64,
    pub block_hash: String,
    pub sum_fee: i64,
    pub transaction_number: i64,
    pub success_rate: i64

    // id: ID!
    // block_number: Int!
    // block_hash: String!
    // sum_fee: Int!
    // transaction_number: Int!
    // success_rate: BigDecimal!
}

impl Into<structmap::GenericMap> for Block {
    fn into(self) -> structmap::GenericMap {
        Block::to_genericmap(self.clone())
    }
}

impl Block {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("block".to_string(), self.clone().into());
        }
    }
}

#[derive(Default, Clone, FromMap, ToMap)]
pub struct Transaction {
    pub id: String,
    pub signature: String,
    pub timestamp: i64,
    pub fee: i64,
    pub block: String,
    pub block_number: i64,
    pub success: bool, // Support bool?

    // id: ID!
    // signature: String!
    // timestamp: Int!
    // fee: Int!
    // block: Block!
    // block_number: Int!
    // success: Boolean!
}

impl Into<structmap::GenericMap> for Transaction {
    fn into(self) -> structmap::GenericMap {
        Transaction::to_genericmap(self.clone())
    }
}

impl Transaction {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("transaction".to_string(), self.clone().into());
        }
    }
}