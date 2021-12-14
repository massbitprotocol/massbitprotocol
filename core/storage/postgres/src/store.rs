use std::sync::Arc;

// use graph::{
//     components::{
//         server::index_node::VersionInfo,
//         store::{BlockStore as BlockStoreTrait, QueryStoreManager, StatusStore},
//     },
//     constraint_violation,
//     data::subgraph::status,
//     prelude::{
//         tokio, web3::types::Address, BlockPtr, CheapClone, DeploymentHash, QueryExecutionError,
//         StoreError,
//     },
// };
use massbit_common::cheap_clone::CheapClone;
use massbit_common::prelude::async_trait::async_trait;
use massbit_common::prelude::tokio;
use massbit_common::util::task_spawn;
use massbit_data::constraint_violation;
use massbit_data::indexer::DeploymentHash;
use massbit_data::prelude::{QueryExecutionError, QueryTarget, StoreError};
use massbit_data::store::chain::BlockPtr;
use massbit_data::store::{QueryStore as QueryStoreTrait, QueryStoreManager};

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
// #[async_trait]
// impl QueryStoreManager for Store {
//     async fn query_store(
//         &self,
//         target: QueryTarget,
//         for_subscription: bool,
//     ) -> Result<Arc<dyn QueryStoreTrait + Send + Sync>, QueryExecutionError> {
//         let store = self.subgraph_store.cheap_clone();
//         let (store, site, replica) = task_spawn::spawn_blocking_allow_panic(move || {
//             store
//                 .replica_for_query(target, for_subscription)
//                 .map_err(|e| e.into())
//         })
//         .await
//         .map_err(|e| QueryExecutionError::Panic(e.to_string()))
//         .and_then(|x| x)?;
//
//         let chain_store = self.block_store.chain_store(&site.network).ok_or_else(|| {
//             constraint_violation!(
//                 "Subgraphs index a known network, but {} indexes `{}` which we do not know about. This is most likely a configuration error.",
//                 site.deployment,
//                 site.network
//             )
//         })?;
//
//         Ok(Arc::new(QueryStore::new(store, chain_store, site, replica)))
//     }
// }

// #[async_trait]
// impl StatusStore for Store {
//     fn status(&self, filter: status::Filter) -> Result<Vec<status::Info>, StoreError> {
//         let mut infos = self.subgraph_store.status(filter)?;
//         let ptrs = self.block_store.chain_head_pointers()?;
//
//         for info in &mut infos {
//             for chain in &mut info.chains {
//                 chain.chain_head_block = ptrs.get(&chain.network).map(|ptr| ptr.to_owned().into());
//             }
//         }
//         Ok(infos)
//     }
//
//     fn version_info(&self, version_id: &str) -> Result<VersionInfo, StoreError> {
//         let mut info = self.subgraph_store.version_info(version_id)?;
//
//         info.total_ethereum_blocks_count = self.block_store.chain_head_block(&info.network)?;
//
//         Ok(info)
//     }
//
//     fn versions_for_subgraph_id(
//         &self,
//         subgraph_id: &str,
//     ) -> Result<(Option<String>, Option<String>), StoreError> {
//         self.subgraph_store.versions_for_subgraph_id(subgraph_id)
//     }
//
//     fn get_proof_of_indexing<'a>(
//         self: Arc<Self>,
//         subgraph_id: &'a DeploymentHash,
//         indexer: &'a Option<Address>,
//         block: BlockPtr,
//     ) -> graph::prelude::DynTryFuture<'a, Option<[u8; 32]>> {
//         self.subgraph_store
//             .get_proof_of_indexing(subgraph_id, indexer, block)
//     }
//
//     async fn query_permit(&self) -> tokio::sync::OwnedSemaphorePermit {
//         // Status queries go to the primary shard.
//         self.block_store.query_permit_primary().await
//     }
// }
