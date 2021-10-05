use serde::{Deserialize, Serialize};
use serde_json;
use std::error::Error;

use crate::types::LightEthereumBlockExt;
use anyhow::Context;
use graph_chain_ethereum::trigger::{
    EthereumBlockData, EthereumEventData, EthereumTransactionData, EthereumTrigger,
};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use web3::types::{Block, Log, Transaction, TransactionReceipt, H256, U256};

use graph::blockchain::{Blockchain, DataSource as _};
use graph::log::logger;
use graph::prelude::Logger;
use graph_chain_ethereum::chain::BlockFinality;
use graph_chain_ethereum::{Chain, DataSource, MappingTrigger};

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

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct ExtTransaction {
    pub version: String,
    pub timestamp: Date,
    pub transaction: Transaction,
    pub receipt: Option<TransactionReceipt>,
}

#[derive(Debug)]
pub struct ExtEvent {
    pub version: String,
    pub timestamp: Date,
    pub event: EthereumEventData,
}

pub fn decode(payload: &mut Vec<u8>) -> Result<EthereumBlock, Box<dyn Error>> {
    let block: EthereumBlock = serde_json::from_slice(&payload).unwrap();
    Ok(block)
}

pub fn get_events(
    eth_block: &EthereumBlock,
    data_source: &DataSource,
    logger: &Logger,
) -> Vec<EthereumEvent> {
    let block = Arc::new(eth_block.block.clone());
    // let logger = graph::log::logger(true);
    eth_block
        .logs
        .iter()
        .filter_map(|log| {
            // Copy from: https://github.com/massbitprotocol/massbit-graph-node/blob/main/chain/ethereum/src/data_source.rs#L491
            let transaction = if log.transaction_hash != block.hash {
                block
                    .transaction_for_log(&log)
                    .context("Found no transaction for event")
            } else {
                // Infer some fields from the log and fill the rest with zeros.
                Ok(Transaction {
                    hash: log.transaction_hash.unwrap(),
                    block_hash: block.hash,
                    block_number: block.number,
                    transaction_index: log.transaction_index,
                    ..Transaction::default()
                })
            };
            // Todo: clean up for optimize
            // Get params
            let trigger: <Chain as Blockchain>::TriggerData =
                EthereumTrigger::Log(Arc::new(log.clone()));

            let block_finality: Arc<<Chain as Blockchain>::Block> =
                Arc::new(BlockFinality::Final(block.clone()));
            let mapping_trigger = data_source
                .match_and_decode(&trigger, block_finality, &logger)
                .unwrap();

            let params = match mapping_trigger {
                Some(MappingTrigger::Log {
                         block: _,
                         transaction: _,
                         log: _,
                         params,
                         handler: _,
                     }) => params,
                _ => Vec::new(),
            };

            // Create EthereumEvent
            match transaction {
                Ok(transaction) => {
                    let transaction = Arc::new(transaction);
                    Some(EthereumEvent {
                        version: eth_block.version.clone(),
                        timestamp: eth_block.timestamp,
                        event: EthereumEventData {
                            address: log.address,
                            log_index: log.log_index.unwrap_or(U256::zero()),
                            transaction_log_index: log.log_index.unwrap_or(U256::zero()),
                            log_type: log.log_type.clone(),
                            block: EthereumBlockData::from(block.as_ref()),
                            transaction: EthereumTransactionData::from(transaction.deref()),
                            params,
                        },
                    })
                }
                Err(_) => None,
            }
        })
        .collect()
}