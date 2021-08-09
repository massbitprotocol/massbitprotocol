use crate::STORE;
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(Default, Clone, FromMap, ToMap)]
pub struct TransactionSolanaTs {
    pub block_number: i64,
    pub fee: i64,
    pub signature: String,
}

impl Into<structmap::GenericMap> for TransactionSolanaTs {
    fn into(self) -> structmap::GenericMap {
        TransactionSolanaTs::to_genericmap(self.clone())
    }
}

impl TransactionSolanaTs {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("TransactionSolanaTs".to_string(), self.clone().into());
        }
    }
}
