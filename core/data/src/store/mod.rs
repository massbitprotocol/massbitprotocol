pub mod chain;
pub mod deployment;
pub mod entity;
pub mod event;
pub mod scalar;
pub mod value;

use crate::prelude::{QueryExecutionError, QueryTarget};
use crate::schema::Schema;
use crate::store::deployment::{DeploymentHash, IndexerName};
pub use event::*;
use massbit_common::prelude::{anyhow::Error, async_trait::async_trait};
use massbit_common::util::MovingStats;
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
    async fn is_deployment_synced(&self) -> Result<bool, Error>;
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
        target: QueryTarget,
        for_subscription: bool,
    ) -> Result<Arc<dyn QueryStore + Send + Sync>, QueryExecutionError>;
}
