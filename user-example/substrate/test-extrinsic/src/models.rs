use crate::STORE;
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(FromMap)]
#[derive(Default, Clone, ToMap)]
pub struct SubstrateExtrinsic {
    pub id: String,
    pub block_number: i64,
    pub extrinsic: String,
}

impl Into<structmap::GenericMap> for SubstrateExtrinsic {
    fn into(self) -> structmap::GenericMap {
        SubstrateExtrinsic::to_genericmap(self.clone())
    }
}

impl SubstrateExtrinsic {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("substrate_extrinsic".to_string(), self.clone().into());
        }
    }
}