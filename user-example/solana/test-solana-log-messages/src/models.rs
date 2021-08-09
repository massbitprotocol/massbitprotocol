use crate::STORE;
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(Default, Clone, FromMap, ToMap)]
pub struct LogMessagesSolanaTs {
    pub block_number: i64,
    pub log_messages: String,
    pub signature: String,
}

impl Into<structmap::GenericMap> for LogMessagesSolanaTs {
    fn into(self) -> structmap::GenericMap {
        LogMessagesSolanaTs::to_genericmap(self.clone())
    }
}

impl LogMessagesSolanaTs {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("LogMessagesSolanaTs".to_string(), self.clone().into());
        }
    }
}
