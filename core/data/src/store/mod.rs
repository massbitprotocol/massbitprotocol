pub mod chain;
pub mod entity;
pub mod event;
pub mod scalar;
pub mod value;

use crate::indexer::error::IndexerError;
use crate::indexer::{DeploymentHash, DeploymentState, IndexerName};
use crate::metrics::stopwatch::StopwatchMetrics;
use crate::prelude::{q, QueryExecutionError, QueryTarget};
use crate::schema::{ApiSchema, Schema};
pub use crate::store::chain::{BlockNumber, BlockPtr};
pub use crate::store::entity::{Entity, EntityKey, EntityModification, EntityQuery, EntityType};
pub use event::*;
use massbit_common::prelude::{anyhow::Error, async_trait::async_trait, serde_json, tokio};
use massbit_common::util::MovingStats;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;
pub use value::Value;

// The type that the connection pool uses to track wait times for
// connection checkouts
pub type PoolWaitStats = Arc<RwLock<MovingStats>>;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("store error: {0}")]
    Unknown(Error),
    #[error(
        "tried to set entity of type `{0}` with ID \"{1}\" but an entity of type `{2}`, \
         which has an interface in common with `{0}`, exists with the same ID"
    )]
    ConflictingId(String, String, String), // (entity, id, conflicting_entity)
    #[error("unknown field '{0}'")]
    UnknownField(String),
    #[error("unknown table '{0}'")]
    UnknownTable(String),
    #[error("malformed directive '{0}'")]
    MalformedDirective(String),
    #[error("query execution failed: {0}")]
    QueryExecutionError(String),
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),
    /// An internal error where we expected the application logic to enforce
    /// some constraint, e.g., that indexer names are unique, but found that
    /// constraint to not hold
    #[error("internal constraint violated: {0}")]
    ConstraintViolation(String),
    #[error("deployment not found: {0}")]
    DeploymentNotFound(String),
    #[error("shard not found: {0} (this usually indicates a misconfiguration)")]
    UnknownShard(String),
    #[error("Fulltext search not yet deterministic")]
    FulltextSearchNonDeterministic,
    #[error("operation was canceled")]
    Canceled,
    #[error("database unavailable")]
    DatabaseUnavailable,
}

// Convenience to report a constraint violation
#[macro_export]
macro_rules! constraint_violation {
    ($msg:expr) => {{
        StoreError::ConstraintViolation(format!("{}", $msg))
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        StoreError::ConstraintViolation(format!($fmt, $($arg)*))
    }}
}

impl From<::diesel::result::Error> for StoreError {
    fn from(e: ::diesel::result::Error) -> Self {
        StoreError::Unknown(e.into())
    }
}

impl From<::diesel::r2d2::PoolError> for StoreError {
    fn from(e: ::diesel::r2d2::PoolError) -> Self {
        StoreError::Unknown(e.into())
    }
}

impl From<Error> for StoreError {
    fn from(e: Error) -> Self {
        StoreError::Unknown(e)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        StoreError::Unknown(e.into())
    }
}

impl From<QueryExecutionError> for StoreError {
    fn from(e: QueryExecutionError) -> Self {
        StoreError::QueryExecutionError(e.to_string())
    }
}
/// Common trait for store implementations.
#[async_trait]
pub trait IndexerStore: Send + Sync + 'static {
    // /// Create a new deployment for the indexer `name`. If the deployment
    // /// already exists (as identified by the `schema.id`), reuse that, otherwise
    // /// create a new deployment, and point the current or pending version of
    // /// `name` at it, depending on the `mode`
    // fn create_indexer_deployment(
    //     &self,
    //     name: IndexerName,
    //     schema: &Schema,
    //     deployment: IndexerDeploymentEntity,
    //     network: String,
    // ) -> Result<DeploymentLocator, StoreError>;

    // /// Return a `WritableStore` that is used for indexer. Only
    // /// code that is part of indexing a indexer should ever use this.
    // fn writable(
    //     &self,
    //     deployment: &DeploymentLocator,
    // ) -> Result<Arc<dyn WritableStore>, StoreError>;

    /// Create a new indexer with the given name. If one already exists, use
    /// the existing one. Return the `id` of the newly created or existing
    /// indexer
    fn create_indexer(&self, name: IndexerName) -> Result<String, StoreError>;

    /// Return `true` if a indexer `name` exists, regardless of whether the
    /// indexer has any deployments attached to it
    fn indexer_exists(&self, name: &IndexerName) -> Result<bool, StoreError>;

    /// Return the GraphQL schema supplied by the user
    fn input_schema(&self, indexer_id: &DeploymentHash) -> Result<Arc<Schema>, StoreError>;

    // Find the deployment locators for the subgraph with the given hash
    //fn locators(&self, hash: &str) -> Result<Vec<DeploymentLocator>, StoreError>;
}
/// Store operations used when serving queries for a specific deployment
#[async_trait]
pub trait QueryStore: Send + Sync {
    fn find_query_values(
        &self,
        query: EntityQuery,
    ) -> Result<Vec<BTreeMap<String, q::Value>>, QueryExecutionError>;

    async fn is_deployment_synced(&self) -> Result<bool, Error>;

    fn block_ptr(&self) -> Result<Option<BlockPtr>, Error>;

    //fn block_number(&self, block_hash: &String) -> Result<Option<BlockNumber>, StoreError>;

    fn wait_stats(&self) -> &PoolWaitStats;

    /// If `block` is `None`, assumes the latest block.
    async fn has_non_fatal_errors(&self, block: Option<BlockNumber>) -> Result<bool, StoreError>;

    /// Find the current state for the subgraph deployment `id` and
    /// return details about it needed for executing queries
    async fn deployment_state(&self) -> Result<DeploymentState, QueryExecutionError>;

    fn api_schema(&self) -> Result<Arc<ApiSchema>, QueryExecutionError>;

    fn network_name(&self) -> &str;

    // A permit should be acquired before starting query execution.
    async fn query_permit(&self) -> tokio::sync::OwnedSemaphorePermit;
}
#[async_trait]
pub trait WritableStore: Send + Sync + 'static {
    /// Get a pointer to the most recently processed block in the subgraph.
    fn block_ptr(&self) -> Result<Option<BlockPtr>, Error>;

    ///// Start an existing subgraph deployment.
    //fn start_subgraph_deployment(&self, logger: &Logger) -> Result<(), StoreError>;

    /// Revert the entity changes from a single block atomically in the store, and update the
    /// subgraph block pointer to `block_ptr_to`.
    ///
    /// `block_ptr_to` must point to the parent block of the subgraph block pointer.
    fn revert_block_operations(&self, block_ptr_to: BlockPtr) -> Result<(), StoreError>;

    /// Remove the fatal error from a subgraph and check if it is healthy or unhealthy.
    fn unfail(&self) -> Result<(), StoreError>;

    /// Set subgraph status to failed with the given error as the cause.
    async fn fail_subgraph(&self, error: IndexerError) -> Result<(), StoreError>;

    /// Looks up an entity using the given store key at the latest block.
    fn get(&self, key: EntityKey) -> Result<Option<Entity>, QueryExecutionError>;

    /// Transact the entity changes from a single block atomically into the store, and update the
    /// subgraph block pointer to `block_ptr_to`.
    ///
    /// `block_ptr_to` must point to a child block of the current subgraph block pointer.
    fn transact_block_operations(
        &self,
        block_ptr_to: BlockPtr,
        mods: Vec<EntityModification>,
        stopwatch: StopwatchMetrics,
        deterministic_errors: Vec<IndexerError>,
    ) -> Result<(), StoreError>;

    /// Look up multiple entities as of the latest block. Returns a map of
    /// entities by type.
    fn get_many(
        &self,
        ids_for_type: BTreeMap<&EntityType, Vec<&str>>,
    ) -> Result<BTreeMap<EntityType, Vec<Entity>>, StoreError>;

    /// The deployment `id` finished syncing, mark it as synced in the database
    /// and promote it to the current version in the subgraphs where it was the
    /// pending version so far
    fn deployment_synced(&self) -> Result<(), Error>;

    /// Return true if the deployment with the given id is fully synced,
    /// and return false otherwise. Errors from the store are passed back up
    async fn is_deployment_synced(&self) -> Result<bool, Error>;

    //fn unassign_subgraph(&self) -> Result<(), StoreError>;

    ///// Load the dynamic data sources for the given deployment
    //async fn load_dynamic_data_sources(&self) -> Result<Vec<StoredDynamicDataSource>, StoreError>;
}
#[async_trait]
pub trait QueryStoreManager: Send + Sync + 'static {
    /// Get a new `QueryStore`. A `QueryStore` is tied to a DB replica, so if Graph Node is
    /// configured to use secondary DB servers the queries will be distributed between servers.
    ///
    /// The query store is specific to a deployment, and `id` must indicate
    /// which deployment will be queried. It is not possible to use the id of the
    /// metadata subgraph, though the resulting store can be used to query
    /// metadata about the deployment `id` (but not metadata about other deployments).
    ///
    /// If `for_subscription` is true, the main replica will always be used.
    async fn query_store(
        &self,
        hash: DeploymentHash,
        for_subscription: bool,
    ) -> Result<Arc<dyn QueryStore + Send + Sync>, QueryExecutionError>;
}
