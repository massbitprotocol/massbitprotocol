use std::sync::Arc;

use massbit::blockchain::{Blockchain, BlockchainKind, BlockchainMap};
use massbit::components::store::{DeploymentId, DeploymentLocator, IndexerStore};
use massbit::data::indexer::schema::IndexerDeploymentEntity;
use massbit::data::indexer::MAX_SPEC_VERSION;
use massbit::data::indexer::{
    CreateIndexerResponse, IndexerManifestResolveError, IndexerManifestValidationError,
    IndexerRegistrarError, UnvalidatedIndexerManifest,
};
use massbit::prelude::LinkResolver;
use massbit::prelude::{
    IndexerAssignmentProvider as IndexerAssignmentProviderTrait,
    IndexerRegistrar as IndexerRegistrarTrait, *,
};

pub struct IndexerRegistrar<L, S, P> {
    resolver: Arc<L>,
    store: Arc<S>,
    chains: Arc<BlockchainMap>,
    node_id: NodeId,
    provider: Arc<P>,
}

impl<L, S, P> IndexerRegistrar<L, S, P>
where
    L: LinkResolver + Clone,
    S: IndexerStore,
    P: IndexerAssignmentProviderTrait,
{
    pub fn new(
        node_id: NodeId,
        chains: Arc<BlockchainMap>,
        resolver: Arc<L>,
        store: Arc<S>,
        provider: Arc<P>,
    ) -> Self {
        IndexerRegistrar {
            resolver: Arc::new(resolver.as_ref().clone().with_retries()),
            store,
            chains,
            node_id,
            provider,
        }
    }
}

#[async_trait]
impl<L, S, P> IndexerRegistrarTrait for IndexerRegistrar<L, S, P>
where
    L: LinkResolver,
    S: IndexerStore,
    P: IndexerAssignmentProvider,
{
    async fn create_indexer(
        &self,
        name: IndexerName,
        hash: DeploymentHash,
        node_id: NodeId,
    ) -> Result<CreateIndexerResponse, Error> {
        let id = self.store.create_indexer(name.clone())?;

        let raw: serde_yaml::Mapping = {
            let file_bytes = self.resolver.cat(&hash.to_ipfs_link()).await.map_err(|e| {
                IndexerRegistrarError::ResolveError(IndexerManifestResolveError::ResolveError(e))
            })?;

            serde_yaml::from_slice(&file_bytes)
                .map_err(|e| IndexerRegistrarError::ResolveError(e.into()))?
        };

        let kind = BlockchainKind::from_manifest(&raw).map_err(|e| {
            IndexerRegistrarError::ResolveError(IndexerManifestResolveError::ResolveError(e))
        })?;

        match kind {
            BlockchainKind::Ethereum => {
                create_indexer_version::<chain_ethereum::Chain, _, _>(
                    self.store.clone(),
                    self.chains.cheap_clone(),
                    name.clone(),
                    hash.cheap_clone(),
                    raw,
                    node_id,
                    self.resolver.cheap_clone(),
                )
                .await?
            }
        };

        self.provider
            .start(DeploymentLocator::new(DeploymentId(1), hash))
            .await;

        Ok(CreateIndexerResponse { id })
    }
}

async fn create_indexer_version<C: Blockchain, S: IndexerStore, L: LinkResolver>(
    store: Arc<S>,
    chains: Arc<BlockchainMap>,
    name: IndexerName,
    deployment: DeploymentHash,
    raw: serde_yaml::Mapping,
    node_id: NodeId,
    resolver: Arc<L>,
) -> Result<(), IndexerRegistrarError> {
    let unvalidated = UnvalidatedIndexerManifest::<C>::resolve(
        deployment,
        raw,
        resolver,
        MAX_SPEC_VERSION.clone(),
    )
    .map_err(IndexerRegistrarError::ResolveError)
    .await?;

    let manifest = unvalidated
        .validate(store.cheap_clone())
        .map_err(IndexerRegistrarError::ManifestValidationError)?;

    let network_name = manifest.network_name();

    let chain = chains
        .get::<C>(network_name.clone())
        .map_err(IndexerRegistrarError::NetworkNotSupported)?
        .cheap_clone();

    let store = store.clone();
    let deployment_store = store.clone();

    if !store.indexer_exists(&name)? {
        return Err(IndexerRegistrarError::NameNotFound(name.to_string()));
    }

    let start_block = resolve_indexer_chain_blocks(&manifest, chain).await?;

    // Apply the indexer versioning and deployment operations,
    // creating a new indexer deployment if one doesn't exist.
    let deployment = IndexerDeploymentEntity::new(&manifest, false, start_block);
    deployment_store
        .create_indexer_deployment(name, &manifest.schema, deployment, node_id, network_name)
        .map_err(|e| IndexerRegistrarError::IndexerDeploymentError(e))
        .map(|_| ())
}

/// Resolves the indexer's earliest block and the manifest's graft base block
async fn resolve_indexer_chain_blocks(
    manifest: &IndexerManifest<impl Blockchain>,
    chain: Arc<impl Blockchain>,
) -> Result<Option<BlockPtr>, IndexerRegistrarError> {
    // If the minimum start block is 0 (i.e. the genesis block),
    // return `None` to start indexing from the genesis block. Otherwise
    // return a block pointer for the block with number `min_start_block - 1`.
    let start_block_ptr = match manifest
        .start_blocks()
        .into_iter()
        .min()
        .expect("cannot identify minimum start block because there are no data sources")
    {
        0 => None,
        min_start_block => chain
            .block_pointer_from_number(min_start_block - 1)
            .await
            .map(Some)
            .map_err(move |_| {
                IndexerRegistrarError::ManifestValidationError(vec![
                    IndexerManifestValidationError::BlockNotFound(min_start_block.to_string()),
                ])
            })?,
    };

    Ok(start_block_ptr)
}

async fn start_indexer(
    deployment: DeploymentLocator,
    provider: Arc<impl IndexerAssignmentProviderTrait>,
) {
    let result = provider.start(deployment).await;
    match result {
        Ok(()) => (),
        Err(e) => {}
    }
}
