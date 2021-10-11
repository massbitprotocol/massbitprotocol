use crate::ethereum::handler::EthereumHandler;
use crate::relational::{Column, ColumnType, Table};
use crate::storage_adapter::StorageAdapter;
use massbit::prelude::web3::types::Transaction;
use massbit::prelude::{Attribute, Entity, Value};
use massbit_common::NetworkType;
use std::collections::HashMap;
use std::sync::Arc;
// use index_store::{EntityValue, FromEntity, FromValueTrait, ToMap, ValueFrom};
// use massbit_drive::{FromEntity, ToMap};
use crate::{create_columns, create_entity};
use massbit::prelude::LightEthereumBlock;

pub struct EthereumRawTransactionHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl EthereumRawTransactionHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        EthereumRawTransactionHandler {
            network: network.clone(),
            storage_adapter,
        }
    }
}

impl EthereumHandler for EthereumRawTransactionHandler {
    fn handle_block(&self, block: Arc<LightEthereumBlock>) -> Result<(), anyhow::Error> {
        let values = block
            .transactions
            .iter()
            .map(|tran| create_entity(block.clone(), tran))
            .collect::<Vec<Entity>>();
        let table = create_table();
        self.storage_adapter.upsert(&table, &values, &None)
    }
}

fn create_table<'a>() -> Table<'a> {
    let columns = create_columns!(
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
    );
    Table::new("ethereum_transaction", columns, Some("t"))
}
fn create_entity(block: Arc<LightEthereumBlock>, trans: &Transaction) -> Entity {
    //let _tran_receipt = block.receipts.get(&trans.hash);
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
