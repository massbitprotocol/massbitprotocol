use crate::STORE;
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(Default, Clone, FromMap, ToMap)]
pub struct ExtrinsicTs {
    pub block_number: i64,
    pub extrinsic: String,
}

impl Into<structmap::GenericMap> for ExtrinsicTs {
    fn into(self) -> structmap::GenericMap {
        ExtrinsicTs::to_genericmap(self.clone())
    }
}

impl ExtrinsicTs {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_ref()
                .unwrap()
                .save("ExtrinsicTs".to_string(), self.clone().into());
        }
    }
}
