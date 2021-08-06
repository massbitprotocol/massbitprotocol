use crate::STORE;
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(FromMap)]
#[derive(Default, Clone, ToMap)]
pub struct SerumBlock {
    pub id: String,
    pub block_number: i64,
    pub block_hash: String,
    pub sum_fee: i64,
    pub transaction_number: i64,
    pub success_rate: i64,
}

impl Into<structmap::GenericMap> for SerumBlock {
    fn into(self) -> structmap::GenericMap {
        SerumBlock::to_genericmap(self.clone())
    }
}

impl SerumBlock {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("serum_block".to_string(), self.clone().into());
        }
    }
}
#[derive(Default, Clone, ToMap)]
pub struct SerumInstructionDetail {
    pub id: String,
    pub name: String,
    pub is_decoded: bool,
}

impl Into<structmap::GenericMap> for SerumInstructionDetail {
    fn into(self) -> structmap::GenericMap {
        SerumInstructionDetail::to_genericmap(self.clone())
    }
}

impl SerumInstructionDetail {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("serum_instruction_detail".to_string(), self.clone().into());
        }
    }
}
#[derive(Default, Clone, ToMap)]
pub struct SerumTransaction {
    pub id: String,
    pub signatures: String,
    pub timestamp: i64,
    pub fee: i64,
    pub block: String,
    pub block_number: i64,
    pub success: bool,
}

impl Into<structmap::GenericMap> for SerumTransaction {
    fn into(self) -> structmap::GenericMap {
        SerumTransaction::to_genericmap(self.clone())
    }
}

impl SerumTransaction {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("serum_transaction".to_string(), self.clone().into());
        }
    }
}
#[derive(Default, Clone, ToMap)]
pub struct SerumTransactionAccount {
    pub id: String,
    pub pub_key: String,
    pub pos_balance: i64,
    pub change_balance: i64,
    pub is_program: bool,
    pub transaction_own: String,
    pub inner_account_index: i64,
}

impl Into<structmap::GenericMap> for SerumTransactionAccount {
    fn into(self) -> structmap::GenericMap {
        SerumTransactionAccount::to_genericmap(self.clone())
    }
}

impl SerumTransactionAccount {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("serum_transaction_account".to_string(), self.clone().into());
        }
    }
}
#[derive(Default, Clone, ToMap)]
pub struct SerumTransactionInstruction {
    pub id: String,
    pub transaction_own: String,
    pub inner_account_index: i64,
    pub instruction_detail: String,
}

impl Into<structmap::GenericMap> for SerumTransactionInstruction {
    fn into(self) -> structmap::GenericMap {
        SerumTransactionInstruction::to_genericmap(self.clone())
    }
}

impl SerumTransactionInstruction {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("serum_transaction_instruction".to_string(), self.clone().into());
        }
    }
}
