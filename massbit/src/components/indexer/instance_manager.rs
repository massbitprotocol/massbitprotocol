use std::sync::Arc;

use crate::components::store::DeploymentLocator;

/// A `IndexerInstanceManager` loads and manages indexer instances.
///
/// When a indexer is added, the subgraph instance manager creates and starts
/// a indexer instances for the indexer. When a indexer is removed, the
/// indexer instance manager stops and removes the corresponding instance.
#[async_trait::async_trait]
pub trait IndexerInstanceManager: Send + Sync + 'static {
    async fn start_indexer(
        self: Arc<Self>,
        deployment: DeploymentLocator,
        manifest: serde_yaml::Mapping,
    );
}
