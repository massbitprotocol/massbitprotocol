use log::{info, warn};
use serde::{Deserialize, Serialize};
use serde_json;
use std::error::Error;

use web3::{
    futures::{future, StreamExt},
    types::{
        Address, Block, BlockId, BlockNumber as Web3BlockNumber, Bytes, CallRequest, Filter,
        FilterBuilder, Log, Transaction, TransactionReceipt, H256,
    },
};

//***************** Ethereum data type *****************
pub type EthereumBlock = ExtBlock;
// pub type EthereumTransaction = ExtTransaction;
//***************** End Ethereum data type *****************

type Number = u32;
type Date = u64;
pub type LightEthereumBlock = Block<Transaction>;

// Similar to
// https://github.com/subquery/subql/blob/93afc96d7ee0ff56d4dd62d8a145088f5bb5e3ec/packages/types/src/interfaces.ts#L18
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct ExtBlock {
    pub version: String,
    pub timestamp: Date,
    pub block: LightEthereumBlock,
    pub receipts: Vec<TransactionReceipt>,
}

pub fn decode(payload: &mut Vec<u8>) -> Result<EthereumBlock, Box<dyn Error>> {
    let decode_block: EthereumBlock = serde_json::from_slice(&payload).unwrap();
    Ok(decode_block)
}
