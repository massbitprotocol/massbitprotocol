use crate::ethereum::handler::EthereumHandler;
use massbit_chain_ethereum::data_type::ExtBlock;
use graph::prelude::web3::types::{Transaction, TransactionReceipt};
use graph::prelude::{Entity, Value, BigInt, BigDecimal as BigDecimalValue, Attribute, chrono};
use crate::storage_adapter::StorageAdapter;
use std::sync::Arc;
use std::collections::HashMap;
use massbit_common::NetworkType;
use bigdecimal::BigDecimal;
use massbit_common::prelude::bigdecimal::FromPrimitive;
use crate::util::timestamp_round_to_date;
use std::time::{Duration, UNIX_EPOCH};
use crate::postgres_queries::UpsertConflictFragment;
use crate::relational::{Table, Column, ColumnType};
use graph::prelude::chrono::Utc;
use crate::{create_columns, create_entity};
use massbit_chain_ethereum::types::LightEthereumBlock;

pub struct EthereumDailyAddressTransactionHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl EthereumDailyAddressTransactionHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        EthereumDailyAddressTransactionHandler {
            network: network.clone(),
            storage_adapter
        }
    }
}

impl EthereumHandler for EthereumDailyAddressTransactionHandler {
    fn handle_block(&self, block: &LightEthereumBlock) -> Result<(), anyhow::Error> {
        let values = block.transactions.iter().map(|tran| {
            //DailyAddressTransactionModel::from(tran).into()
            create_entity(block,tran)
        }).collect::<Vec<Entity>>();
        let table = Table::new("ethereum_daily_address_transaction", Some("t"));
        let columns = create_columns();
        let mut conflict_frag = UpsertConflictFragment::new("ethereum_daily_address_transaction_date_uindex");
        conflict_frag.add_expression("transaction_count", "t.transaction_count + EXCLUDED.transaction_count")
            .add_expression("transaction_volume","t.transaction_volume + EXCLUDED.transaction_volume")
            .add_expression("gas","t.gas + EXCLUDED.gas");
        self.storage_adapter.upsert(&table,
                                    &columns,
                                    &values,
                                    Some(conflict_frag));
        Ok(())
    }
}
fn create_columns() -> Vec<Column> {
    create_columns!(
        "address" => ColumnType::String,
        "transaction_date" => ColumnType::Varchar,
        "transaction_count" => ColumnType::BigInt,
        "transaction_volume" => ColumnType::BigDecimal,
        "gas" => ColumnType::BigDecimal
    )
}
fn create_entity(block: &LightEthereumBlock, transaction: &Transaction) -> Entity {
    let time = UNIX_EPOCH + Duration::from_secs(block.timestamp.as_u64());
    // Create DateTime from SystemTime
    let datetime = chrono::DateTime::<Utc>::from(time).format("%Y-%m-%d").to_string();
    create_entity!(
        "address" => transaction.from,
        "transaction_date" => datetime,
        "transaction_count" => 1_u64,
        "transaction_volume" => transaction.value,
        "gas" => transaction.gas
    )
}
