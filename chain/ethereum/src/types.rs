use std::collections::HashMap;
use std::convert::TryFrom;
use web3::types::{Block, Log, Transaction, TransactionReceipt, H256};

pub type LightEthereumBlock = Block<Transaction>;

#[derive(Debug)]
pub struct EthereumBlock {
    pub version: String,
    pub timestamp: u64,
    pub block: LightEthereumBlock,
    pub receipts: HashMap<H256, TransactionReceipt>,
    pub logs: Vec<Log>,
}

impl TryFrom<Vec<u8>> for EthereumBlock {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        serde_json::from_slice(&value)?
    }
}

#[derive(Debug)]
pub struct EthereumTransaction {
    pub version: String,
    pub timestamp: u64,
    pub transaction: Transaction,
    pub receipt: Option<TransactionReceipt>,
}

#[derive(Debug)]
pub struct EthereumEvent {
    pub version: String,
    pub timestamp: u64,
    pub event: EthereumEventData,
}
