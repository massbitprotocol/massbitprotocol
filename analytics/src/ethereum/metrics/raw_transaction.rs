use massbit_common::prelude::bigdecimal::{BigDecimal, FromPrimitive};
use graph::prelude::web3::types::Transaction;
use massbit_common::NetworkType;
use crate::storage_adapter::StorageAdapter;
use std::sync::Arc;
use crate::ethereum::handler::EthereumHandler;
use crate::relational::{Table, Column, ColumnType};
use graph::prelude::Entity;
use index_store::{EntityValue, FromEntity, FromValueTrait, ToMap, ValueFrom};
use massbit_drive::{FromEntity, ToMap};

pub struct EthereumRawTransactionHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl EthereumRawTransactionHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        EthereumRawTransaction {
            network: network.clone(),
            storage_adapter
        }
    }
}

impl EthereumHandler for EthereumRawTransactionHandler {
    fn handle_transactions(&self, transactions: &Vec<Transaction>) -> Result<(), anyhow::Error> {
        let values = transactions.iter().map(|tran| {
            DailyAddressTransactionModel::from(tran).into()
        }).collect::<Vec<Entity>>();
        let table = Table::new("ethereum_transaction", Some("t"));
        let columns = vec![
            Column::new("transaction_hash", ColumnType::String),
            Column::new("block_hash", ColumnType::Varchar),
            Column::new("block_number", ColumnType::BigInt),
            Column::new("nonce", ColumnType::BigDecimal),
            Column::new("sender", ColumnType::String),
            Column::new("receiver", ColumnType::String),
            Column::new("value", ColumnType::BigDecimal),
            Column::new("gas_limit", ColumnType::BigDecimal),
            Column::new("gas_price", ColumnType::BigDecimal),
            Column::new("timestamp", ColumnType::BigInt),
        ];
        self.storage_adapter.upsert(&table,
                                    &columns,
                                    &values,
                                    None);
        Ok(())
    }
}

fn create_columns() -> Vec<Column> {
    create_columns!(
        "block_hash" => ColumnType::String,
        "parent_hash" => ColumnType::String,
        "block_number" => ColumnType::BigInt,
        "transaction_number" => ColumnType::BigInt,
        "timestamp" => ColumnType::BigInt,
        "validated_by" => ColumnType::String,
        "reward" => ColumnType::BigInt,
        "difficulty" => ColumnType::BigInt,
        "total_difficulty" => ColumnType::BigInt,
        "size" => ColumnType::BigInt,
        "gas_used" => ColumnType::BigDecimal,
        "gas_limit" => ColumnType::BigDecimal,
        "extra_data" => ColumnType::Bytes
    )
}
fn create_entity(block: &LightEthereumBlock) -> Entity {
    create_entity!(
        "block_hash" => block.hash,
        "parent_hash" => block.parent_hash,
        "block_number" => block.number,
        "transaction_number" => block.transactions.len() as u64,
        "timestamp" => block.timestamp,
        "validated_by" => block.author,
        "reward" => 0_64,
        "difficulty" => block.difficulty,
        "total_difficulty" => block.total_difficulty,
        "size" => block.size,
        "gas_used" => block.gas_used,
        "gas_limit" => block.gas_limit,
        "extra_data" => block.extra_data.clone()
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