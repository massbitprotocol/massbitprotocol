use crate::STORE;
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(FromMap)]
#[derive(Default, Clone, ToMap)]
pub struct Block {
    pub id: String,
    pub block_height: i64,
    pub block_hash: String,
    pub timestamp: String,
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