use crate::STORE;
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(Default, Clone, FromMap, ToMap)]
pub struct BlockTs {
    pub id: String,
    pub block_height: i64,
}

impl Into<structmap::GenericMap> for BlockTs {
    fn into(self) -> structmap::GenericMap {
        BlockTs::to_genericmap(self.clone())
    }
}

impl BlockTs {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_ref()
                .unwrap()
                .save("BlockTs".to_string(), self.clone().into());
        }
    }
}