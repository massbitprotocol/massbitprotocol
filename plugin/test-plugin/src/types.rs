use massbit_runtime_wasm::graph::data::store::scalar::BigInt;
use massbit_runtime_wasm::prelude::web3::ethabi::Address;
use std::str::FromStr;

pub struct Block {}

impl Block {
    pub fn default() -> Block {
        Block {}
    }
}

pub struct Transaction {}
impl Transaction {
    pub fn default() -> Transaction {
        Transaction {}
    }
}
pub struct EventParam {}
impl EventParam {
    pub fn default() -> EventParam {
        EventParam {}
    }
}
pub struct PairCreated {
    address: Address,
    logIndex: BigInt,
    transactionLogIndex: BigInt,
    logType: String,
    block: Block,
    transaction: Transaction,
    parameters: Vec<EventParam>,
}
impl PairCreated {
    pub fn new(addr: &str) -> PairCreated {
        PairCreated {
            address: Address::from_str(addr).unwrap(),
            logIndex: BigInt::from(1),
            transactionLogIndex: BigInt::from(2),
            logType: String::from("logType"),
            block: Block::default(),
            transaction: Transaction::default(),
            parameters: Vec::default(),
        }
    }
}
