// use crate::types::LightEthereumBlock;
// use massbit::prelude::*;
// use std::collections::HashMap;
// use std::convert::TryFrom;
// use web3::types::{Log, Transaction, TransactionReceipt, H256};
//
// #[derive(Debug)]
// pub struct EthereumBlock {
//     pub version: String,
//     pub timestamp: u64,
//     pub block: LightEthereumBlock,
//     pub receipts: HashMap<H256, TransactionReceipt>,
//     pub logs: Vec<Log>,
// }
//
// impl TryFrom<Vec<u8>> for EthereumBlock {
//     type Error = Error;
//
//     fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
//         serde_json::from_slice(&value)?
//     }
// }
