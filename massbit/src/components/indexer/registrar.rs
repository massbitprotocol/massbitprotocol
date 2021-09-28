use async_trait::async_trait;

use crate::data::indexer::CreateIndexerResponse;
use crate::prelude::*;

/// Common trait for subgraph registrars.
#[async_trait]
pub trait IndexerRegistrar: Send + Sync + 'static {
    async fn create_indexer(
        &self,
        name: IndexerName,
        hash: DeploymentHash,
    ) -> Result<CreateIndexerResponse, Error>;
}
