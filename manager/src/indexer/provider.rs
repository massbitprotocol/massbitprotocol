use std::collections::HashSet;
use std::sync::Mutex;

use async_trait::async_trait;

use massbit::{
    components::store::{DeploymentId, DeploymentLocator},
    prelude::{IndexerProvider as IndexerProviderTrait, *},
};

pub struct IndexerProvider<I> {
    logger_factory: LoggerFactory,
    indexers_running: Arc<Mutex<HashSet<DeploymentId>>>,
    instance_manager: Arc<I>,
}

impl<I> IndexerProvider<I>
where
    I: IndexerInstanceManager,
{
    pub fn new(logger_factory: &LoggerFactory, instance_manager: I) -> Self {
        let logger = logger_factory.component_logger("IndexerAssignmentProvider");
        let logger_factory = logger_factory.with_parent(logger.clone());

        // Create the indexer provider
        IndexerProvider {
            logger_factory,
            indexers_running: Arc::new(Mutex::new(HashSet::new())),
            instance_manager: Arc::new(instance_manager),
        }
    }
}

#[async_trait]
impl<I> IndexerProviderTrait for IndexerProvider<I>
where
    I: IndexerInstanceManager,
{
    async fn start(
        &self,
        loc: DeploymentLocator,
        manifest: serde_yaml::Mapping,
    ) -> Result<(), IndexerProviderError> {
        let logger = self.logger_factory.indexer_logger(&loc);

        // If indexer ID already in set
        if !self.indexers_running.lock().unwrap().insert(loc.id) {
            info!(logger, "Indexer deployment is already running");

            return Err(IndexerProviderError::AlreadyRunning(loc.hash.clone()));
        }

        self.instance_manager
            .cheap_clone()
            .start_indexer(loc.clone(), manifest)
            .await;

        Ok(())
    }
}
