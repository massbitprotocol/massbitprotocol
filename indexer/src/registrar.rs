use massbit::components::store::IndexerStore;
use massbit::data::indexer::CreateIndexerResponse;
use massbit::prelude::LinkResolver;
use massbit::prelude::{IndexerRegistrar as IndexerRegistrarTrait, *};
use std::sync::Arc;

pub struct IndexerRegistrar<L, S> {
    resolver: Arc<L>,
    store: Arc<S>,
}

impl<L, S> IndexerRegistrar<L, S>
where
    L: LinkResolver + Clone,
    S: IndexerStore,
{
    pub fn new(resolver: Arc<L>, store: Arc<S>) -> Self {
        IndexerRegistrar {
            resolver: Arc::new(resolver.as_ref().clone().with_retries()),
            store,
        }
    }
}

#[async_trait]
impl<L, S> IndexerRegistrarTrait for IndexerRegistrar<L, S>
where
    L: LinkResolver,
    S: IndexerStore,
{
    async fn create_indexer(
        &self,
        name: IndexerName,
        hash: DeploymentHash,
    ) -> Result<CreateIndexerResponse, Error> {
        let id = self.store.create_indexer(name.clone())?;
        Ok(CreateIndexerResponse { id })
    }
}
