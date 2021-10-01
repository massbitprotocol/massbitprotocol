use futures::future::join_all;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::sync::Arc;

use massbit::prelude::*;
use massbit_store_postgres::connection_pool::{ConnectionPool, ForeignServer, PoolName};
use massbit_store_postgres::{IndexerStore, Shard as ShardName};

use crate::config::{Config, Shard};

pub struct StoreBuilder {
    logger: Logger,
    indexer_store: Arc<IndexerStore>,
    pools: HashMap<ShardName, ConnectionPool>,
}

impl StoreBuilder {
    /// Set up all stores, and run migrations. This does a complete store
    /// setup whereas other methods here only get connections for an already
    /// initialized store
    pub async fn new(logger: &Logger, config: &Config) -> Self {
        let (store, pools) = Self::make_indexer_store_and_pools(logger, config);

        // Try to perform setup (migrations etc.) for all the pools. If this
        // attempt doesn't work for all of them because the database is
        // unavailable, they will try again later in the normal course of
        // using the pool
        join_all(pools.iter().map(|(_, pool)| async move { pool.setup() })).await;

        Self {
            logger: logger.cheap_clone(),
            indexer_store: store,
            pools,
        }
    }

    pub fn store(self) -> Arc<IndexerStore> {
        self.indexer_store
    }

    /// Make a `ShardedStore` across all configured shards, and also return
    /// the main connection pools for each shard, but not any pools for
    /// replicas
    pub fn make_indexer_store_and_pools(
        logger: &Logger,
        config: &Config,
    ) -> (Arc<IndexerStore>, HashMap<ShardName, ConnectionPool>) {
        let servers = config
            .stores
            .iter()
            .map(|(name, shard)| ForeignServer::new_from_raw(name.to_string(), &shard.connection))
            .collect::<Result<Vec<_>, _>>()
            .expect("connection url's contain enough detail");
        let servers = Arc::new(servers);

        let shards: Vec<_> = config
            .stores
            .iter()
            .map(|(name, shard)| {
                let logger = logger.new(o!("shard" => name.to_string()));
                let conn_pool = Self::main_pool(&logger, name, shard, servers.clone());

                let (read_only_conn_pools, weights) =
                    Self::replica_pools(&logger, name, shard, servers.clone());

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

        let store = Arc::new(IndexerStore::new(logger, shards));

        (store, pools)
    }

    /// Create a connection pool for the main database of hte primary shard
    /// without connecting to all the other configured databases
    pub fn main_pool(
        logger: &Logger,
        name: &str,
        shard: &Shard,
        servers: Arc<Vec<ForeignServer>>,
    ) -> ConnectionPool {
        let logger = logger.new(o!("pool" => "main"));
        info!(logger, "Connecting to Postgres");
        ConnectionPool::create(
            name,
            PoolName::Main,
            shard.connection.to_owned(),
            10,
            Some(10),
            servers,
            &logger,
        )
    }

    /// Create connection pools for each of the replicas
    fn replica_pools(
        logger: &Logger,
        name: &str,
        shard: &Shard,
        servers: Arc<Vec<ForeignServer>>,
    ) -> (Vec<ConnectionPool>, Vec<usize>) {
        let mut weights: Vec<_> = vec![shard.weight];
        (
            shard
                .replicas
                .values()
                .enumerate()
                .map(|(i, replica)| {
                    let pool = format!("replica{}", i + 1);
                    info!(&logger, "Connecting to Postgres (read replica {})", i + 1);
                    weights.push(replica.weight);
                    ConnectionPool::create(
                        name,
                        PoolName::Replica(pool),
                        replica.connection.clone(),
                        10,
                        None,
                        servers.clone(),
                        &logger,
                    )
                })
                .collect(),
            weights,
        )
    }
}
