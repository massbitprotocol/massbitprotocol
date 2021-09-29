use std::collections::HashSet;
use std::sync::Mutex;

use async_trait::async_trait;

use massbit::{
    components::store::{DeploymentId, DeploymentLocator},
    prelude::{IndexerAssignmentProvider as IndexerAssignmentProviderTrait, *},
};

pub struct IndexerAssignmentProvider<L, I> {
    indexers_running: Arc<Mutex<HashSet<DeploymentId>>>,
    link_resolver: Arc<L>,
    instance_manager: Arc<I>,
}

impl<L, I> IndexerAssignmentProvider<L, I>
where
    L: LinkResolver + CheapClone,
    I: IndexerInstanceManager,
{
    pub fn new(link_resolver: Arc<L>, instance_manager: I) -> Self {
        // Create the subgraph provider
        IndexerAssignmentProvider {
            indexers_running: Arc::new(Mutex::new(HashSet::new())),
            link_resolver: Arc::new(link_resolver.as_ref().cheap_clone().with_retries()),
            instance_manager: Arc::new(instance_manager),
        }
    }
}

#[async_trait]
impl<L, I> IndexerAssignmentProviderTrait for IndexerAssignmentProvider<L, I>
where
    L: LinkResolver,
    I: IndexerInstanceManager,
{
    async fn start(&self, loc: DeploymentLocator) -> Result<(), IndexerAssignmentProviderError> {
        // If subgraph ID already in set
        if !self.indexers_running.lock().unwrap().insert(loc.id) {
            info!("Indexer deployment is already running");

            return Err(IndexerAssignmentProviderError::AlreadyRunning(
                loc.hash.clone(),
            ));
        }

        let file_bytes = self
            .link_resolver
            .cat(&loc.hash.to_ipfs_link())
            .await
            .map_err(IndexerAssignmentProviderError::ResolveError)?;

        let raw: serde_yaml::Mapping = serde_yaml::from_slice(&file_bytes)
            .map_err(|e| IndexerAssignmentProviderError::ResolveError(e.into()))?;

        self.instance_manager
            .cheap_clone()
            .start_indexer(loc, raw)
            .await;

        Ok(())
    }
}
