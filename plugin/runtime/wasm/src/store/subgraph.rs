use crate::prelude::Arc;
use graph::blockchain::BlockPtr;
use graph::components::store::{
    DeploymentLocator, StoreError, SubgraphStore as SubgraphStoreTrait, WritableStore,
};
use graph::components::subgraph::SubgraphVersionSwitchingMode;
use graph::data::query::QueryExecutionError;
use graph::data::schema::{ApiSchema, Schema};
use graph::data::store::NodeId;
use graph::data::subgraph::schema::SubgraphDeploymentEntity;
use graph::data::subgraph::{DeploymentHash, SubgraphName};
use massbit_common::prelude::anyhow::Error;

pub struct SubgraphStore {}
impl SubgraphStore {
    pub fn new() -> Self {
        SubgraphStore {}
    }
}
impl SubgraphStoreTrait for SubgraphStore {
    fn find_ens_name(&self, _hash: &str) -> Result<Option<String>, QueryExecutionError> {
        todo!()
    }

    fn is_deployed(&self, id: &DeploymentHash) -> Result<bool, Error> {
        todo!()
    }

    fn create_subgraph_deployment(
        &self,
        name: SubgraphName,
        schema: &Schema,
        deployment: SubgraphDeploymentEntity,
        node_id: NodeId,
        network: String,
        mode: SubgraphVersionSwitchingMode,
    ) -> Result<DeploymentLocator, StoreError> {
        todo!()
    }

    fn create_subgraph(&self, name: SubgraphName) -> Result<String, StoreError> {
        todo!()
    }

    fn remove_subgraph(&self, name: SubgraphName) -> Result<(), StoreError> {
        todo!()
    }

    fn reassign_subgraph(
        &self,
        deployment: &DeploymentLocator,
        node_id: &NodeId,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn assigned_node(&self, deployment: &DeploymentLocator) -> Result<Option<NodeId>, StoreError> {
        todo!()
    }

    fn assignments(&self, node: &NodeId) -> Result<Vec<DeploymentLocator>, StoreError> {
        todo!()
    }

    fn subgraph_exists(&self, name: &SubgraphName) -> Result<bool, StoreError> {
        todo!()
    }

    fn input_schema(&self, subgraph_id: &DeploymentHash) -> Result<Arc<Schema>, StoreError> {
        todo!()
    }

    fn api_schema(&self, subgraph_id: &DeploymentHash) -> Result<Arc<ApiSchema>, StoreError> {
        todo!()
    }

    fn writable(
        &self,
        deployment: &DeploymentLocator,
    ) -> Result<Arc<dyn WritableStore>, StoreError> {
        todo!()
    }

    fn writable_for_network_indexer(
        &self,
        id: &DeploymentHash,
    ) -> Result<Arc<dyn WritableStore>, StoreError> {
        todo!()
    }

    fn least_block_ptr(&self, id: &DeploymentHash) -> Result<Option<BlockPtr>, Error> {
        todo!()
    }

    fn locators(&self, hash: &str) -> Result<Vec<DeploymentLocator>, StoreError> {
        todo!()
    }
}
