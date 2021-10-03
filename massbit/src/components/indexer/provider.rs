use async_trait::async_trait;

use crate::{components::store::DeploymentLocator, prelude::*};

/// Common trait for indexer providers.
#[async_trait]
pub trait IndexerProvider: Send + Sync + 'static {
    async fn start(
        &self,
        deployment: DeploymentLocator,
        manifest: serde_yaml::Mapping,
    ) -> Result<(), IndexerProviderError>;
}
