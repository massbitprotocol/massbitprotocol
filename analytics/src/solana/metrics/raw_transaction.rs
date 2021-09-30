//[WIP]
use graph::prelude::{Entity, Attribute, Value};
use massbit_common::NetworkType;
use crate::storage_adapter::StorageAdapter;
use std::sync::Arc;
use crate::relational::{ColumnType, Table, Column};
use std::collections::HashMap;
use graph::data::store::ValueType::BigInt;
use crate::{create_columns,create_entity};
use crate::solana::handler::SolanaHandler;
use massbit_chain_solana::data_type::SolanaBlock;
use solana_transaction_status::{ConfirmedBlock, TransactionWithStatusMeta};
use core::fmt::Display;

pub struct SolanaRawTransactionHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl SolanaRawTransactionHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        SolanaRawTransactionHandler {
            network: network.clone(),
            storage_adapter
        }
    }
}

impl SolanaHandler for SolanaRawTransactionHandler {
    fn handle_block(&self, block: &SolanaBlock) -> Result<(), anyhow::Error> {
        let table = Table::new("solana_transactions", Some("t"));
        let columns = create_columns();
        let mut tran_entities = Vec::default();
        for tran in &block.block.transactions {
            tran_entities.push(create_entity(&block.block, tran));
            //println!("{:?}",tran);
        }
        self.storage_adapter.upsert(&table,
                                    &columns,
                                    &tran_entities,
                                    None);
        Ok(())
    }
}
fn create_columns() -> Vec<Column> {
    create_columns!(
        "signatures" => ColumnType::String,
        "block_number" => ColumnType::BigInt,
        "block_hash" => ColumnType::String,
        "signers" => ColumnType::String,
        "timestamp" => ColumnType::BigInt,
        "reward" => ColumnType::BigInt
    )
}
fn create_entity(block: &ConfirmedBlock, tran: &TransactionWithStatusMeta) -> Entity {
    let timestamp = match block.block_time {
        None => 0_u64,
        Some(val) => val as u64
    };
    //block.rewards.iter().reduce(|reward|{ reward.commission});
    let signatures = tran.transaction.signatures.iter().map(|sig|{
        format!("{:?}", sig)
    }).collect::<Vec<String>>().join(",");
    //let signers = tran.transaction.message.account_keys.
    create_entity!(
        "signatures" => signatures,
        "block_number" => block.parent_slot,
        "block_hash" => block.blockhash.clone(),
        "signers" => String::from(""),
        "timestamp" => timestamp,
        "reward" => 0_u64
    )
}

