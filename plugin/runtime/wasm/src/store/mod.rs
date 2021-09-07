use std::collections::BTreeMap;

use massbit_common::prelude::async_trait::async_trait;

pub use index_store::core::IndexStore;
pub mod postgres;
use graph::components::store::{EntityKey, EntityModification, EntityType, StoreError};
use graph::components::subgraph::Entity;
use graph::data::query::QueryExecutionError;
pub use postgres::PostgresIndexStore;

/*
#[async_trait]
pub trait WritableStore: Send + Sync + 'static {
    /// Looks up an entity using the given store key at the latest block.
    fn get(&self, key: EntityKey) -> Result<Option<Entity>, QueryExecutionError>;
    /// Look up multiple entities as of the latest block. Returns a map of
    /// entities by type.
    fn get_many(
        &self,
        ids_for_type: BTreeMap<&EntityType, Vec<&str>>,
    ) -> Result<BTreeMap<EntityType, Vec<Entity>>, StoreError>;

    /// Transact the entity changes from a single block atomically into the store, and update the
    /// subgraph block pointer to `block_ptr_to`.
    ///
    /// `block_ptr_to` must point to a child block of the current subgraph block pointer.
    fn transact_block_operations(
        &self,
        //block_ptr_to: BlockPtr,
        mods: Vec<EntityModification>,
        //stopwatch: StopwatchMetrics,
        //data_sources: Vec<StoredDynamicDataSource>,
        //deterministic_errors: Vec<SubgraphError>,
    ) -> Result<(), StoreError>;
}
*/
