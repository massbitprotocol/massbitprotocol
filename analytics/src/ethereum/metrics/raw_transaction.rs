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
    fn handle_block(&self, block: &LightEthereumBlock) -> Result<(), anyhow::Error> {
        let values = block.transactions.iter().map(|tran| {
            create_entity(block, tran)
        }).collect::<Vec<Entity>>();
        let table = Table::new("ethereum_transaction", Some("t"));
        let columns = create_columns();
        self.storage_adapter.upsert(&table,
                                    &columns,
                                    &values,
                                    None);
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
fn create_entity(block: &LightEthereumBlock, trans: &Transaction) -> Entity {
    create_entity!(
        "transaction_hash" => trans.hash,
        "block_hash" => trans.block_hash,
        "block_number" => trans.block_number,
        "nonce" => trans.nonce,
        "sender" => trans.from,
        "receiver" => trans.to,
        "value" => trans.value,
        "gas" => trans.gas,
        "gas_price" => trans.gas_price,
        "timestamp" => block.timestamp
    )
}
// #[derive(Default, Debug, Clone, FromEntity, ToMap)]
// pub struct EthereumTransaction {
//     pub transaction_hash: String,
//     pub block_hash: String,
//     pub block_number: i64,
//     pub nonce: BigDecimal,
//     pub sender: String,
//     pub receiver: String,
//     pub value: BigDecimal,
//     pub gas_limit: BigDecimal,
//     pub gas_price: BigDecimal,
//     pub timestamp: i64
// }
//
// impl  From<&Transaction> for EthereumTransaction {
//     fn from(trans: &Transaction) -> Self {
//         let transaction_hash = format!("{:x}",trans.hash);
//         let block_hash = match trans.block_hash {
//             None => None,
//             Some(hash) => Some(format!("{:x}",hash)),
//         };
//         let block_number = match trans.block_number {
//             Some(val) => Some(val.as_u64() as i64),
//             _ => None
//         };
//         let sender = format!("{:x}",trans.from);
//         let receiver = match trans.to {
//             Some(val) => Some(format!("{:x}",val)),
//             _ => None
//         };
//         let value = match BigDecimal::from_u128(trans.value.as_u128()) {
//             None => BigDecimal::from(0),
//             Some(val) => val
//         };
//         let gas_limit = match BigDecimal::from_u128(trans.gas.as_u128()) {
//             None => BigDecimal::from(0),
//             Some(val) => val
//         };
//         let gas_price = match BigDecimal::from_u128(trans.gas_price.as_u128()) {
//             None => BigDecimal::from(0),
//             Some(val) => val
//         };
//         EthereumTransaction {
//             transaction_hash,
//             block_hash,
//             block_number,
//             nonce: BigDecimal::from_u128(trans.value.as_u128()),
//             sender,
//             receiver,
//             value,
//             gas_limit,
//             gas_price,
//             timestamp: 0,
//         }
//     }
// }