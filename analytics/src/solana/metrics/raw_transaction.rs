//[WIP]
use crate::models::CommandData;
use crate::relational::{Column, ColumnType, Table};
use crate::solana::handler::SolanaHandler;
use crate::solana::model::AccountTrans;
use crate::storage_adapter::StorageAdapter;
use crate::{create_columns, create_entity};
use core::fmt::Display;
use graph::data::store::ValueType::BigInt;
use graph::prelude::{Attribute, Entity, Value};
use massbit_chain_solana::data_type::SolanaBlock;
use massbit_common::NetworkType;
use solana_transaction_status::{ConfirmedBlock, TransactionWithStatusMeta};
use std::collections::HashMap;
use std::sync::Arc;

pub struct SolanaRawTransactionHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl SolanaRawTransactionHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        SolanaRawTransactionHandler {
            network: network.clone(),
            storage_adapter,
        }
    }
}

impl SolanaHandler for SolanaRawTransactionHandler {
    fn handle_block(&self, block: Arc<SolanaBlock>) -> Result<(), anyhow::Error> {
        let tran_table = Table::new("solana_transactions", Some("t"));
        let trans_columns = create_trans_columns();
        let mut tran_entities = Vec::default();
        let mut acc_tran_entities = Vec::default();
        for tran in &block.block.transactions {
            tran_entities.push(create_entity(&block.block, tran));
            //Create account trans list
            let tx_hash = tran
                .transaction
                .signatures
                .get(0)
                .and_then(|sig| Some(sig.to_string()));
            let mut tx_accounts = create_transaction_account(&tx_hash, tran);
            acc_tran_entities.extend(tx_accounts);
        }
        let acc_tran_table = Table::new("solana_account_transactions", Some("t"));
        let acc_trans_columns = create_acc_trans_columns();
        let trans_data = CommandData::new(&tran_table, &trans_columns, &tran_entities, &None);
        let acc_trans_data = CommandData::new(
            &acc_tran_table,
            &acc_trans_columns,
            &acc_tran_entities,
            &None,
        );
        self.storage_adapter
            .transact_upserts(vec![trans_data, acc_trans_data]);
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
        Some(val) => val as u64,
    };
    // let signatures = tran.transaction.signatures.iter().map(|sig|{
    //     format!("{:?}", sig)
    // }).collect::<Vec<String>>();
    let mut tx_hash = String::from("");
    if let Some(sig) = tran.transaction.signatures.get(0) {
        tx_hash = format!("{:?}", sig);
    }
    let mut signers: Vec<String> = Vec::default();
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
            Err(_) => "Error".to_string(),
        }
    }
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

fn create_transaction_account(
    tx_hash: &Option<String>,
    tran: &TransactionWithStatusMeta,
) -> Vec<Entity> {
    let hash = tx_hash.clone().unwrap_or_default();
    tran.transaction
        .message
        .account_keys
        .iter()
        .enumerate()
        .map(|(ind, key)| {
            let pre_balance = tran
                .meta
                .as_ref()
                .and_then(|meta| meta.pre_balances.get(ind))
                .unwrap_or(&0_u64);
            let post_balance = tran
                .meta
                .as_ref()
                .and_then(|meta| meta.post_balances.get(ind))
                .unwrap_or(&0_u64);
            create_entity!(
                "tx_hash" => hash.clone(),
                "account" => format!("{:?}", key),
                "pre_balance" => *pre_balance,
                "post_balance" => *post_balance
            )
        })
        .collect::<Vec<Entity>>()
}
