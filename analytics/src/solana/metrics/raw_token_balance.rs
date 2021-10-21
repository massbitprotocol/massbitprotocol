use crate::relational::{Column, ColumnType, Table};
use crate::solana::handler::SolanaHandler;
use crate::storage_adapter::StorageAdapter;
use crate::{create_columns, create_entity};
use massbit::data::store::scalar::BigInt;
use massbit::prelude::{Attribute, Entity, Error, Value};
use massbit_chain_solana::data_type::SolanaBlock;
use massbit_common::NetworkType;
use solana_transaction_status::{
    EncodedConfirmedBlock, EncodedTransactionWithStatusMeta, TransactionStatusMeta,
    TransactionWithStatusMeta, UiTransactionStatusMeta,
};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

pub struct SolanaTokenBalanceHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl SolanaTokenBalanceHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        SolanaTokenBalanceHandler {
            network: network.clone(),
            storage_adapter,
        }
    }
}

impl SolanaHandler for SolanaTokenBalanceHandler {
    fn handle_block(
        &self,
        block_slot: u64,
        block: Arc<EncodedConfirmedBlock>,
    ) -> Result<(), Error> {
        let table = create_table();
        let entities = block
            .transactions
            .iter()
            .enumerate()
            .filter_map(|(tran_order, tran)| {
                tran.meta.as_ref().and_then(|meta| {
                    Some(create_token_balances(
                        tran,
                        meta,
                        block_slot,
                        tran_order as i32,
                    ))
                })
            })
            .reduce(|mut a, mut b| {
                a.append(&mut b);
                a
            });
        if let Some(values) = entities {
            if values.len() > 0 {
                self.storage_adapter.upsert(&table, &values, &None);
            }
        }
        Ok(())
    }
}

fn create_table<'a>() -> Table<'a> {
    let columns = create_columns!(
        "block_slot" => ColumnType::BigInt,
        "tx_index" => ColumnType::Int,
        "account" => ColumnType::String,
        "token_address" => ColumnType::String,
        "decimals" => ColumnType::Int,
        "pre_amount" => ColumnType::BigInt,
        "post_amount" => ColumnType::BigInt
    );
    Table::new("solana_token_balances", columns)
}

fn create_token_balances(
    tran: &EncodedTransactionWithStatusMeta,
    meta: &UiTransactionStatusMeta,
    block_slot: u64,
    tran_index: i32,
) -> Vec<Entity> {
    // let tx_hash = match tran.transaction.signatures.get(0) {
    //     Some(sig) => format!("{:?}", sig),
    //     None => String::from(""),
    // };
    let decoded_tran = tran.transaction.decode();
    if meta.pre_token_balances.is_some()
        && meta.post_token_balances.is_some()
        && decoded_tran.is_some()
    {
        meta.post_token_balances
            .as_ref()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(ind, token_balance)| {
                let account = decoded_tran
                    .as_ref()
                    .unwrap()
                    .message
                    .account_keys
                    .get(token_balance.account_index as usize)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default();
                // println!("Post token balance {:?}", token_balance);
                // println!(
                //     "Pre token balance {:?}",
                //     meta.pre_token_balances.as_ref().unwrap().get(ind)
                // );
                let post_amount = BigInt::from_str(token_balance.ui_token_amount.amount.as_str())
                    .unwrap_or(BigInt::from(0_u64));
                let pre_amount = meta
                    .pre_token_balances
                    .as_ref()
                    .unwrap()
                    .get(ind)
                    .and_then(|token_balance| {
                        BigInt::from_str(token_balance.ui_token_amount.amount.as_str()).ok()
                    })
                    .unwrap_or(BigInt::from(0_i32));
                create_entity!(
                    "block_slot" => block_slot,
                    "tx_index" => tran_index,
                    "account" => account,
                    "token_address" => token_balance.mint.clone(),
                    "decimals" => token_balance.ui_token_amount.decimals as i32,
                    "pre_amount" => pre_amount,
                    "post_amount" => post_amount
                )
            })
            .collect::<Vec<Entity>>()
    } else {
        Vec::default()
    }
}
