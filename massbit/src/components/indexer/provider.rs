use async_trait::async_trait;

use crate::{components::store::DeploymentLocator, prelude::*};

/// Common trait for indexer providers.
#[async_trait]
pub trait IndexerAssignmentProvider: Send + Sync + 'static {
    async fn start(
        &self,
        deployment: DeploymentLocator,
    ) -> Result<(), IndexerAssignmentProviderError>;
}
