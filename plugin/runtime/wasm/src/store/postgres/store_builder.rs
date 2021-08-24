use crate::chain::ethereum::EthereumNetworkIdentifier;
use crate::graph::cheap_clone::CheapClone;
use crate::graph::components::metrics::MetricsRegistry;
use crate::prelude::{Arc, Logger};
use crate::store::postgres::ConnectionPool;
use crate::store::{PostgresIndexStore, ShardConfig, ShardName};
use crate::util::security::SafeDisplay;
use slog::{info, o};
use std::collections::HashMap;

pub struct StoreBuilder {}

impl StoreBuilder {
    pub fn create_store(
        indexer: String,
        shard_name: ShardName,
        connection_string: String,
    ) -> PostgresIndexStore {
        //let connection_pool = StoreBuilder::main_pool();
        PostgresIndexStore {
            connection_string,
            //connection_pool,
        }
    }
    /// Create a connection pool for the main database of hte primary shard
    /// without connecting to all the other configured databases
    pub fn main_pool(
        logger: &Logger,
        indexer: &String,
        name: &str,
        shard: &ShardConfig,
        registry: Arc<dyn MetricsRegistry>,
    ) -> ConnectionPool {
        let pool_size = shard.pool_size.size_for(indexer, name).expect(&format!(
            "we can determine the pool size for store {}",
            name
        ));
        let fdw_pool_size = shard.fdw_pool_size.size_for(indexer, name).expect(&format!(
            "we can determine the fdw pool size for store {}",
            name
        ));
        info!(
            logger,
            "Connecting to Postgres";
            "url" => SafeDisplay(shard.connection.as_str()),
            "conn_pool_size" => pool_size,
            "weight" => shard.weight
        );
        ConnectionPool::create(
            name,
            "main",
            shard.connection.to_owned(),
            pool_size,
            Some(fdw_pool_size),
            &logger,
            registry.cheap_clone(),
        )
    }
}
