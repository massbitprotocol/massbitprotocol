use crate::STORE;
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(FromMap)]
#[derive(Default, Clone, ToMap)]
pub struct SubstrateEvent {
    pub id: String,
    pub event: String,
    pub timestamp: String,
}

impl Into<structmap::GenericMap> for SubstrateEvent {
    fn into(self) -> structmap::GenericMap {
        SubstrateEvent::to_genericmap(self.clone())
    }
}

impl SubstrateEvent {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("substrate_event".to_string(), self.clone().into());
        }
    }
}
