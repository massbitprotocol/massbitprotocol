use massbit_common::prelude::async_trait::async_trait;
//use graphql_parser::query::Subscription;
use massbit_data::indexer::DeploymentHash;
use massbit_data::prelude::LoadManager;
use massbit_data::query::{CacheStatus, Query, QueryResults};
use std::sync::Arc;
use std::time::Duration;

// /// Future for subscription results.
// pub type SubscriptionResultFuture =
//     Box<dyn Future<Item = SubscriptionResult, Error = SubscriptionError> + Send>;

pub enum GraphQlTarget {
    SubgraphName(String),
    Deployment(DeploymentHash),
}

/// A component that can run GraphqL queries against a [Store](../store/trait.Store.html).
#[async_trait]
pub trait GraphQlRunner: Send + Sync + 'static {
    /// Runs a GraphQL query and returns its result.
    async fn run_query(self: Arc<Self>, query: Query, hash: DeploymentHash) -> QueryResults;

    /// Runs a GraphqL query up to the given complexity. Overrides the global complexity limit.
    async fn run_query_with_complexity(
        self: Arc<Self>,
        query: Query,
        hash: DeploymentHash,
        max_complexity: Option<u64>,
        max_depth: Option<u8>,
        max_first: Option<u32>,
        max_skip: Option<u32>,
    ) -> QueryResults;

    // /// Runs a GraphQL subscription and returns a stream of results.
    // async fn run_subscription(
    //     self: Arc<Self>,
    //     subscription: Subscription,
    //     target: QueryTarget,
    // ) -> Result<SubscriptionResult, SubscriptionError>;

    fn load_manager(&self) -> Arc<LoadManager>;
}

#[async_trait]
pub trait QueryLoadManager: Send + Sync {
    fn record_work(&self, shape_hash: u64, duration: Duration, cache_status: CacheStatus);
}
