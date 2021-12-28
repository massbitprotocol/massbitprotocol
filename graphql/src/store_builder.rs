use std::iter::FromIterator;
use std::{collections::HashMap, sync::Arc};

use crate::config::{Config, Shard};
use massbit_common::cheap_clone::CheapClone;
use massbit_common::prelude::diesel::{Connection, PgConnection};
use massbit_common::prelude::slog::{info, o, Logger};
use massbit_common::util::security::SafeDisplay;
use massbit_data::indexer::NodeId;
use massbit_data::metrics::MetricsRegistry as MetricsRegistryTrait;
use massbit_storage_postgres::connection_pool::ConnectionPool;
use massbit_storage_postgres::store::StoreManager;
use massbit_storage_postgres::IndexerStore;
use massbit_storage_postgres::Shard as ShardName;

pub struct StoreBuilder {
    logger: Logger,
    indexer_store: Arc<IndexerStore>,
    pools: HashMap<ShardName, ConnectionPool>,
}

impl StoreBuilder {
    /// Set up all stores, and run migrations. This does a complete store
    /// setup whereas other methods here only get connections for an already
    /// initialized store
    pub async fn new(
        logger: &Logger,
        node: &NodeId,
        config: &Config,
        //        connection_pool: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
        registry: Arc<dyn MetricsRegistryTrait>,
    ) -> Self {
        let (store, pools) =
            Self::make_indexer_store_and_pools(logger, node, config, registry.cheap_clone());
        Self {
            logger: logger.cheap_clone(),
            indexer_store: store,
            pools,
        }
    }
    /// Make a `ShardedStore` across all configured shards, and also return
    /// the main connection pools for each shard, but not any pools for
    /// replicas
    pub fn make_indexer_store_and_pools(
        logger: &Logger,
        node: &NodeId,
        config: &Config,
        registry: Arc<dyn MetricsRegistryTrait>,
    ) -> (Arc<IndexerStore>, HashMap<ShardName, ConnectionPool>) {
        let shards: Vec<_> = config
            .stores
            .iter()
            .map(|(name, shard)| {
                let logger = logger.new(o!("shard" => name.to_string()));
                let conn_pool = Self::main_pool(&logger, node, name, shard, registry.cheap_clone());

                let (read_only_conn_pools, weights) =
                    Self::replica_pools(&logger, node, name, shard, registry.cheap_clone());

                let name =
                    ShardName::new(name.to_string()).expect("shard names have been validated");
                (name, conn_pool, read_only_conn_pools, weights)
            })
            .collect();

        let pools: HashMap<_, _> = HashMap::from_iter(
            shards
                .iter()
                .map(|(name, pool, _, _)| (name.clone(), pool.clone())),
        );

        let store = Arc::new(IndexerStore::new(
            logger,
            shards,
            Arc::new(config.deployment.clone()),
        ));

        (store, pools)
    }
    /// Create a connection pool for the main database of hte primary shard
    /// without connecting to all the other configured databases
    pub fn main_pool(
        logger: &Logger,
        node: &NodeId,
        name: &str,
        shard: &Shard,
        registry: Arc<dyn MetricsRegistryTrait>,
    ) -> ConnectionPool {
        let logger = logger.new(o!("pool" => "main"));
        let pool_size = shard.pool_size.size_for(node, name).expect(&format!(
            "we can determine the pool size for store {}",
            name
        ));
        let fdw_pool_size = shard.fdw_pool_size.size_for(node, name).expect(&format!(
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
    /// Create connection pools for each of the replicas
    fn replica_pools(
        logger: &Logger,
        node: &NodeId,
        name: &str,
        shard: &Shard,
        registry: Arc<dyn MetricsRegistryTrait>,
    ) -> (Vec<ConnectionPool>, Vec<usize>) {
        let mut weights: Vec<_> = vec![shard.weight];
        (
            shard
                .replicas
                .values()
                .enumerate()
                .map(|(i, replica)| {
                    let pool = &format!("replica{}", i + 1);
                    let logger = logger.new(o!("pool" => pool.clone()));
                    info!(
                        &logger,
                        "Connecting to Postgres (read replica {})", i+1;
                        "url" => SafeDisplay(replica.connection.as_str()),
                        "weight" => replica.weight
                    );
                    weights.push(replica.weight);
                    let pool_size = replica.pool_size.size_for(node, name).expect(&format!(
                        "we can determine the pool size for replica {}",
                        name
                    ));
                    ConnectionPool::create(
                        name,
                        pool,
                        replica.connection.clone(),
                        pool_size,
                        None,
                        &logger,
                        registry.cheap_clone(),
                    )
                })
                .collect(),
            weights,
        )
    }
    pub async fn store_manager(&self) -> StoreManager {
        StoreManager::new(self.indexer_store.clone())
    }
}
