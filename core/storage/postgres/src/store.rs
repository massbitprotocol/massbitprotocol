use massbit_common::cheap_clone::CheapClone;
use massbit_common::prelude::async_trait::async_trait;
use massbit_common::prelude::tokio;
use massbit_common::util::task_spawn;
use massbit_data::constraint_violation;
use massbit_data::indexer::DeploymentHash;
use massbit_data::prelude::{QueryExecutionError, QueryTarget, StoreError};
use massbit_data::store::chain::BlockPtr;
use massbit_data::store::{QueryStore as QueryStoreTrait, QueryStoreManager};
use std::sync::Arc;

use crate::query_store::QueryStore;
use crate::IndexerStore;

/// The overall store of the system, consisting of a [SubgraphStore] and a
/// [BlockStore], each of which multiplex across multiple database shards.
/// The `SubgraphStore` is responsible for storing all data and metadata related
/// to individual subgraphs, and the `BlockStore` does the same for data belonging
/// to the chains that are being processed.
///
/// This struct should only be used during configuration and setup of `graph-node`.
/// Code that needs to access the store should use the traits from [graph::components::store]
/// and only require the smallest traits that are suitable for their purpose
pub struct StoreManager {
    indexer_store: Arc<IndexerStore>,
}

impl StoreManager {
    pub fn new(indexer_store: Arc<IndexerStore>) -> Self {
        Self { indexer_store }
    }
    pub fn indexer_store(&self) -> Arc<IndexerStore> {
        self.indexer_store.cheap_clone()
    }
}
#[async_trait]
impl QueryStoreManager for StoreManager {
    async fn query_store(
        &self,
        hash: DeploymentHash,
        for_subscription: bool,
    ) -> Result<Arc<dyn QueryStoreTrait + Send + Sync>, QueryExecutionError> {
        let store = self.indexer_store.cheap_clone();
        let (store, site, replica) = task_spawn::spawn_blocking_allow_panic(move || {
            store
                .replica_for_query(hash, for_subscription)
                .map_err(|e| e.into())
        })
        .await
        .map_err(|e| QueryExecutionError::Panic(e.to_string()))
        .and_then(|x| x)?;

        Ok(Arc::new(QueryStore::new(store, site, replica)))
    }
}
