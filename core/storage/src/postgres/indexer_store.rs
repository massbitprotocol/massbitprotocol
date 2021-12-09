use massbit_common::prelude::tokio::sync::OwnedSemaphorePermit;
use massbit_common::prelude::{anyhow::Error, async_trait::async_trait};
use massbit_data::prelude::q::Value;
use massbit_data::prelude::{QueryExecutionError, QueryTarget, StoreError};
use massbit_data::schema::ApiSchema;
use massbit_data::store::chain::{BlockNumber, BlockPtr};
use massbit_data::store::deployment::DeploymentState;
use massbit_data::store::entity::EntityQuery;
use massbit_data::store::{PoolWaitStats, QueryStore as QueryStoreTrait, QueryStoreManager};
use std::collections::BTreeMap;
use std::sync::Arc;

pub struct IndexerStore {}

#[async_trait]
impl QueryStoreManager for IndexerStore {
    async fn query_store(
        &self,
        target: QueryTarget,
        for_subscription: bool,
    ) -> Result<Arc<dyn QueryStoreTrait + Send + Sync>, QueryExecutionError> {
        let query_store = QueryStore {};
        Ok(Arc::new(query_store))
    }
}

pub struct QueryStore {}

#[async_trait]
impl QueryStoreTrait for QueryStore {
    fn find_query_values(
        &self,
        query: EntityQuery,
    ) -> Result<Vec<BTreeMap<String, Value>>, QueryExecutionError> {
        todo!()
    }

    async fn is_deployment_synced(&self) -> Result<bool, Error> {
        todo!()
    }

    fn block_ptr(&self) -> Result<Option<BlockPtr>, Error> {
        todo!()
    }

    fn block_number(&self, block_hash: &String) -> Result<Option<BlockNumber>, StoreError> {
        todo!()
    }

    fn wait_stats(&self) -> &PoolWaitStats {
        todo!()
    }

    async fn has_non_fatal_errors(&self, block: Option<BlockNumber>) -> Result<bool, StoreError> {
        todo!()
    }

    async fn deployment_state(&self) -> Result<DeploymentState, QueryExecutionError> {
        todo!()
    }

    fn api_schema(&self) -> Result<Arc<ApiSchema>, QueryExecutionError> {
        todo!()
    }

    fn network_name(&self) -> &str {
        todo!()
    }

    async fn query_permit(&self) -> OwnedSemaphorePermit {
        todo!()
    }
}
