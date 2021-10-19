use serde::{Deserialize, Serialize};
use solana_transaction_status::EncodedConfirmedBlock;
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct EncodedConfirmedBlockWithSlot {
    pub block_slot: u64,
    pub block: EncodedConfirmedBlock,
}

#[derive(Clone, Debug)]
pub struct Account {
    account: String,
    pub_name: String,
}
#[derive(Clone, Debug, Default)]
pub struct AccountTrans {
    tx_hash: String,
    account: String,
    pre_balance: i64,
    post_balance: i64,
    signer: bool,
    writable: bool,
}
impl AccountTrans {
    pub fn from_tran_account(tx_hash: String, account: String) -> AccountTrans {
        AccountTrans {
            tx_hash,
            account,
            pre_balance: 0,
            post_balance: 0,
            signer: false,
            writable: false,
        }
    }
}
