use serde::{Deserialize, Serialize};
use serde_json;
use std::error::Error;

use std::collections::HashMap;
use web3::types::{
    Address, Block, BlockId, BlockNumber as Web3BlockNumber, Bytes, CallRequest, Filter,
    FilterBuilder, Log, Transaction, TransactionReceipt, H256,
};

//***************** Ethereum data type *****************
pub type EthereumBlock = ExtBlock;
pub type EthereumTransaction = ExtTransaction;
pub type EthereumEvent = ExtEvent;
//***************** End Ethereum data type *****************

type Date = u64;
pub type LightEthereumBlock = Block<Transaction>;

// Similar to
// https://github.com/subquery/subql/blob/93afc96d7ee0ff56d4dd62d8a145088f5bb5e3ec/packages/types/src/interfaces.ts#L18
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct ExtBlock {
    pub version: String,
    pub timestamp: Date,
    pub block: LightEthereumBlock,
    pub receipts: HashMap<H256, TransactionReceipt>,
    pub logs: Vec<Log>,
}

pub struct ExtTransaction {
    pub version: String,
    pub timestamp: Date,
    pub transaction: Transaction,
    pub receipt: Option<TransactionReceipt>,
}

pub struct ExtEvent {
    pub version: String,
    pub timestamp: Date,
    pub logs: Vec<Log>,
}

// pub struct Event {
//     pub address: Address,
//     pub logIndex: i64,
//     pub transactionLogIndex: i64,
//     pub logType: Option<String>,
//     pub block: Block,
//     pub transaction: Transaction,
//     pub parameters: Array<EventParam>,
// }

pub fn decode(payload: &mut Vec<u8>) -> Result<EthereumBlock, Box<dyn Error>> {
    let block: EthereumBlock = serde_json::from_slice(&payload).unwrap();
    Ok(block)
}
