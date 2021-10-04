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
use crate::solana::model::AccountTrans;
use crate::models::CommandData;
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
        let tran_table = Table::new("solana_transactions", Some("t"));
        let trans_columns = create_trans_columns();
        let mut tran_entities = Vec::default();
        let mut acc_tran_entities = Vec::default();
        for tran in &block.block.transactions {
            tran_entities.push(create_entity(&block.block, tran));
            //Create account trans list
            let mut tx_hash = String::from("");
            if let Some(sig) = tran.transaction.signatures.get(0) {
                tx_hash = format!("{:?}", sig);
            }
            let mut acc_tx_entities = tran.transaction.message.account_keys.iter().map(|key|{
                create_entity!(
                    "tx_hash" => tx_hash.clone(),
                    "account" => format!("{:?}", key),
                    "pre_balance" => 0_u64,
                    "post_balance" => 0_u64
                )
            }).collect::<Vec<Entity>>();
            acc_tran_entities.extend(acc_tx_entities);
        }
        let acc_tran_table = Table::new("solana_account_transactions", Some("t"));
        let acc_trans_columns = create_acc_trans_columns();
        let trans_data = CommandData::new(&tran_table, &trans_columns, &tran_entities, &None);
        let acc_trans_data = CommandData::new(&acc_tran_table,
                                                    &acc_trans_columns,
                                              &acc_tran_entities,
                                              &None);
        self.storage_adapter.transact_upserts(vec![trans_data, acc_trans_data]);
        // self.storage_adapter.upsert(&tran_table,
        //                             &tran_columns,
        //                             &tran_entities,
        //                             None);
        Ok(())
    }
}
fn create_trans_columns() -> Vec<Column> {
    create_columns!(
        "signatures" => ColumnType::String,
        "block_number" => ColumnType::BigInt,
        "parent_slot" => ColumnType::BigInt,
        "block_hash" => ColumnType::String,
        "signers" => ColumnType::String,
        "block_time" => ColumnType::BigInt,
        "reward" => ColumnType::BigInt,
        "fee" => ColumnType::BigInt,
        "status" => ColumnType::String
    )
}
fn create_acc_trans_columns() -> Vec<Column> {
    create_columns!(
        "tx_hash" => ColumnType::String,
        "account" => ColumnType::String,
        "pre_balance" => ColumnType::BigInt,
        "post_balance" => ColumnType::BigInt
    )
}
fn create_entity(block: &ConfirmedBlock, tran: &TransactionWithStatusMeta) -> Entity {
    let timestamp = match block.block_time {
        None => 0_u64,
        Some(val) => val as u64
    };
    // let signatures = tran.transaction.signatures.iter().map(|sig|{
    //     format!("{:?}", sig)
    // }).collect::<Vec<String>>();
    let mut tx_hash = String::from("");
    if let Some(sig) = tran.transaction.signatures.get(0) {
        tx_hash = format!("{:?}", sig);
    }
    let mut signers : Vec<String> = Vec::default();
    for i in 0..tran.transaction.signatures.len() {
        if let Some(key) = tran.transaction.message.account_keys.get(i) {
            signers.push(format!("{:?}", key));
        }
    }

    let mut tran_fee = 0_u64;
    let mut tran_status = String::default();
    if let Some(meta) = &tran.meta {
        tran_fee = meta.fee;
        tran_status = match meta.status {
            Ok(_) => "Success".to_string(),
            Err(_) => "Error".to_string()
        }
    }
    //panic!();
    create_entity!(
        "signatures" => tx_hash,
        "block_number" => block.block_height,
        "parent_slot" => block.parent_slot,
        "block_hash" => block.blockhash.clone(),
        "signers" => signers.join(","),
        "block_time" => timestamp,
        "status" => tran_status,
        "reward" => 0_u64,
        "fee" => tran_fee
    )
}
