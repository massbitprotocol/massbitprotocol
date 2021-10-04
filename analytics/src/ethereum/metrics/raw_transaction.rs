use massbit_common::prelude::bigdecimal::{BigDecimal, FromPrimitive};
use graph::prelude::web3::types::Transaction;
use massbit_common::NetworkType;
use crate::storage_adapter::StorageAdapter;
use std::sync::Arc;
use std::collections::HashMap;
use crate::ethereum::handler::EthereumHandler;
use crate::relational::{Table, Column, ColumnType};
use graph::prelude::{Attribute, Entity, Value};
// use index_store::{EntityValue, FromEntity, FromValueTrait, ToMap, ValueFrom};
// use massbit_drive::{FromEntity, ToMap};
use massbit_chain_ethereum::types::LightEthereumBlock;
use crate::{create_columns, create_entity};
use massbit_chain_ethereum::data_type::ExtBlock;

pub struct EthereumRawTransactionHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl EthereumRawTransactionHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        EthereumRawTransactionHandler {
            network: network.clone(),
            storage_adapter
        }
    }
}

impl EthereumHandler for EthereumRawTransactionHandler {
    fn handle_block(&self, block: &ExtBlock) -> Result<(), anyhow::Error> {
        let values = block.block.transactions.iter().map(|tran| {
            create_entity(block, tran)
        }).collect::<Vec<Entity>>();
        let table = Table::new("ethereum_transaction", Some("t"));
        let columns = create_columns();
        self.storage_adapter.upsert(&table,
                                    &columns,
                                    &values,
                                    &None);
        Ok(())
    }
}

fn create_columns() -> Vec<Column> {
    create_columns!(
        "transaction_hash" => ColumnType::String,
        "block_hash" => ColumnType::Varchar,
        "block_number" => ColumnType::BigInt,
        "nonce" => ColumnType::BigDecimal,
        "sender" => ColumnType::String,
        "receiver" => ColumnType::String,
        "value" => ColumnType::BigDecimal,
        "gas" => ColumnType::BigDecimal,
        "gas_price" => ColumnType::BigDecimal,
        "timestamp" => ColumnType::BigInt
    )
}
fn create_entity(block: &ExtBlock, trans: &Transaction) -> Entity {
    let tran_receipt = block.receipts.get(&trans.hash);
    //tran_receipt.and_then(|r|r.status).
    create_entity!(
        "transaction_hash" => trans.hash,
        "block_hash" => trans.block_hash,
        "block_number" => trans.block_number,
        "nonce" => trans.nonce,
        "sender" => trans.from,
        //"status" => tran_receipt.
        "receiver" => trans.to,
        "value" => trans.value,
        "gas" => trans.gas,
        "gas_price" => trans.gas_price,
        "timestamp" => block.timestamp
    )
}
