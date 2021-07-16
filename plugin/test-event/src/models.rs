use crate::STORE;
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(Default, Clone, FromMap, ToMap)]
pub struct EventTs {
    pub event: String,
    pub timestamp: String,
}

impl Into<structmap::GenericMap> for EventTs {
    fn into(self) -> structmap::GenericMap {
        EventTs::to_genericmap(self.clone())
    }
}

impl EventTs {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_ref()
                .unwrap()
                .save("EventTs".to_string(), self.clone().into());
        }
    }
}
