use crate::STORE;
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(Default, Clone, FromMap, ToMap)]
pub struct BlockSolanaTs {
    pub block_hash: String,
    pub block_height: i64,
}

impl Into<structmap::GenericMap> for BlockSolanaTs {
    fn into(self) -> structmap::GenericMap {
        BlockSolanaTs::to_genericmap(self.clone())
    }
}

impl BlockSolanaTs {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_ref()
                .unwrap()
                .save("BlockSolanaTs".to_string(), self.clone().into());
        }
    }
}
