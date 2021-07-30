use crate::STORE;
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(FromMap)]
#[derive(Default, Clone, ToMap)]
pub struct SubstrateBlock {
    pub id: String,
    pub block_hash: String,
    pub block_height: i64,
}

impl Into<structmap::GenericMap> for SubstrateBlock {
    fn into(self) -> structmap::GenericMap {
        SubstrateBlock::to_genericmap(self.clone())
    }
}

impl SubstrateBlock {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("substrate_block".to_string(), self.clone().into());
        }
    }
}