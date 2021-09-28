use lru_time_cache::LruCache;
use massbit::prelude::{ApiSchema, DeploymentHash, Schema};
use rand::{seq::SliceRandom, thread_rng};
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

use crate::connection_pool::ConnectionPool;
use crate::relational::LayoutCache;

/// When connected to read replicas, this allows choosing which DB server to use for an operation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ReplicaId {
    /// The main server has write and read access.
    Main,

    /// A read replica identified by its index.
    ReadOnly(usize),
}

pub struct StoreInner {
    conn: ConnectionPool,
    read_only_pools: Vec<ConnectionPool>,

    /// A list of the available replicas set up such that when we run
    /// through the list once, we picked each replica according to its
    /// desired weight. Each replica can appear multiple times in the list
    replica_order: Vec<ReplicaId>,
    /// The current position in `replica_order` so we know which one to
    /// pick next
    conn_round_robin_counter: AtomicUsize,
}

/// Storage of the data for individual deployments. Each `DeploymentStore`
/// corresponds to one of the database shards that `SubgraphStore` manages.
#[derive(Clone)]
pub struct DeploymentStore(Arc<StoreInner>);

impl DeploymentStore {
    pub fn new(
        pool: ConnectionPool,
        read_only_pools: Vec<ConnectionPool>,
        mut pool_weights: Vec<usize>,
    ) -> Self {
        // Create a list of replicas with repetitions according to the weights
        // and shuffle the resulting list. Any missing weights in the list
        // default to 1
        pool_weights.resize(read_only_pools.len() + 1, 1);
        let mut replica_order: Vec<_> = pool_weights
            .iter()
            .enumerate()
            .map(|(i, weight)| {
                let replica = if i == 0 {
                    ReplicaId::Main
                } else {
                    ReplicaId::ReadOnly(i - 1)
                };
                vec![replica; *weight]
            })
            .flatten()
            .collect();
        let mut rng = thread_rng();
        replica_order.shuffle(&mut rng);

        // Create the store
        let store = StoreInner {
            conn: pool,
            read_only_pools,
            replica_order,
            conn_round_robin_counter: AtomicUsize::new(0),
        };
        let store = DeploymentStore(Arc::new(store));

        // Return the store
        store
    }
}
