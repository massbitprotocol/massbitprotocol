use crate::ethereum::handler::EthereumHandler;
use massbit_chain_ethereum::data_type::ExtBlock;
use graph::prelude::web3::types::{Transaction, TransactionReceipt};
use graph::prelude::{Entity, Value, BigInt, BigDecimal as BigDecimalValue, Attribute};
use crate::storage_adapter::StorageAdapter;
use std::sync::Arc;
use std::collections::HashMap;
use massbit_common::NetworkType;
use index_store::{EntityValue, FromEntity, FromValueTrait, ToMap, ValueFrom};
use massbit_drive::{FromEntity, ToMap};
use bigdecimal::BigDecimal;
use massbit_common::prelude::bigdecimal::FromPrimitive;

pub struct EthereumDailyAddressTransaction {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl EthereumDailyAddressTransaction {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        EthereumDailyAddressTransaction {
            network: network.clone(),
            storage_adapter
        }
    }
}

impl EthereumHandler for EthereumDailyAddressTransaction {
    fn handle_transactions(&self, transactions: &Vec<Transaction>) -> Result<(), anyhow::Error> {

        Ok(())
    }
}

#[derive(Default, Debug, Clone, FromEntity, ToMap)]
pub struct DailyAddressTransactionModel {
    pub address: String,
    pub transaction_date: String,
    pub timestamp: i64,
    pub transaction_count: i64,
    pub transaction_volume: BigDecimalValue,
    pub gas: BigDecimalValue
}

impl Into<Entity> for DailyAddressTransactionModel {
    fn into(self) -> Entity {
        let map = DailyAddressTransactionModel::to_map(self.clone());
        Entity::from(map)
    }
}

impl From<&Transaction> for DailyAddressTransactionModel {
    fn from(transaction: &Transaction) -> Self {
        let parent_hash = format!("{:x}", transaction.parent_hash);
        let hash = format!("{:x}", transaction.hash.unwrap());

        // let transaction_hash = format!(
        //     "0x{}",
        //     hex::encode(trans.hash.as_bytes()).trim_start_matches('0')
        // );
        let block_number = match trans.block_number {
            Some(val) => Some(val.as_u64() as i64),
            _ => None
        };
        let sender = format!("{:x}", trans.from);
        let receiver = match trans.to {
            Some(val) => Some(format!("{:x}",val)),
            _ => None
        };
        let value = match BigDecimal::from_u128(trans.value.as_u128()) {
            None => BigDecimal::from(0),
            Some(val) => val
        };
        let gas = match BigDecimal::from_u128(trans.gas.as_u128()) {
            None => BigDecimal::from(0),
            Some(val) => val
        };
        let gas_price = match BigDecimal::from_u128(trans.gas_price.as_u128()) {
            None => BigDecimal::from(0),
            Some(val) => val
        };
        DailyAddressTransactionModel {
            address: sender,
            transaction_date: "".to_string(),
            timestamp: 0,
            transaction_count: 1,
            transaction_volume: BigDecimalValue::from(value),
            gas: BigDecimalValue::from(gas)
        }
    }
}