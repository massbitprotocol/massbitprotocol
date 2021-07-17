use crate::STORE;
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(Default, Clone, FromMap, ToMap)]
pub struct Block {
    pub block_hash: String,
    pub block_height: i64,
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
                .as_ref()
                .unwrap()
                .save("Block".to_string(), self.clone().into());
        }
    }
}
