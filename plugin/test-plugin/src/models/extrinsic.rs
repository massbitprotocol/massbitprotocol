use crate::STORE;
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(Default, Clone, FromMap, ToMap)]
pub struct Extrinsic {
    pub id: String,
    pub block_hash: String,
    pub block_height: i64,
    pub origin: String,
}

impl Into<structmap::GenericMap> for Extrinsic {
    fn into(self) -> structmap::GenericMap {
        Extrinsic::to_genericmap(self.clone())
    }
}

impl Extrinsic {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_ref()
                .unwrap()
                .save("Extrinsic".to_string(), self.clone().into());
        }
    }
}
