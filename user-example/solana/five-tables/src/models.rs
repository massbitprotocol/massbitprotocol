use crate::STORE;
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(FromMap)]
#[derive(Default, Clone, ToMap)]
pub struct Block {
    pub id: String,
    pub block_number: i64,
    pub block_hash: String,
    pub sum_fee: i64,
    pub transaction_number: i64,
    pub success_rate: i64,
}

impl Into<structmap::GenericMap> for Block {
    fn into(self) -> structmap::GenericMap {
        Block::to_genericmap(self.clone())
    }
}

impl Block {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("Block".to_string(), self.clone().into());
        }
    }
}
#[derive(Default, Clone, ToMap)]
pub struct InstructionDetail {
    pub id: String,
    pub name: String,
    pub is_decoded: bool,
}

impl Into<structmap::GenericMap> for InstructionDetail {
    fn into(self) -> structmap::GenericMap {
        InstructionDetail::to_genericmap(self.clone())
    }
}

impl InstructionDetail {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("InstructionDetail".to_string(), self.clone().into());
        }
    }
}
#[derive(Default, Clone, ToMap)]
pub struct Transaction {
    pub id: String,
    pub signatures: String,
    pub timestamp: i64,
    pub fee: i64,
    pub block_number: i64,
    pub success: bool,
}

impl Into<structmap::GenericMap> for Transaction {
    fn into(self) -> structmap::GenericMap {
        Transaction::to_genericmap(self.clone())
    }
}

impl Transaction {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("Transaction".to_string(), self.clone().into());
        }
    }
}
#[derive(Default, Clone, ToMap)]
pub struct TransactionAccount {
    pub id: String,
    pub pub_key: String,
    pub pos_balance: i64,
    pub change_balance: i64,
    pub is_program: bool,
    pub transaction_own: String,
    pub inner_account_index: i64,
}

impl Into<structmap::GenericMap> for TransactionAccount {
    fn into(self) -> structmap::GenericMap {
        TransactionAccount::to_genericmap(self.clone())
    }
}

impl TransactionAccount {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("TransactionAccount".to_string(), self.clone().into());
        }
    }
}
#[derive(Default, Clone, ToMap)]
pub struct TransactionInstruction {
    pub id: String,
    pub transaction_own: String,
    pub inner_account_index: i64,
    pub instruction_detail: String,
}

impl Into<structmap::GenericMap> for TransactionInstruction {
    fn into(self) -> structmap::GenericMap {
        TransactionInstruction::to_genericmap(self.clone())
    }
}

impl TransactionInstruction {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("TransactionInstruction".to_string(), self.clone().into());
        }
    }
}