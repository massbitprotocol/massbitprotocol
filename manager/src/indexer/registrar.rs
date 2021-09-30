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
    logger: Logger,
    logger_factory: LoggerFactory,
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
        logger_factory: &LoggerFactory,
        node_id: NodeId,
        chains: Arc<BlockchainMap>,
        resolver: Arc<L>,
        store: Arc<S>,
        provider: Arc<P>,
    ) -> Self {
        let logger = logger_factory.component_logger("IndexerRegistrar");
        let logger_factory = logger_factory.with_parent(logger.clone());
        IndexerRegistrar {
            logger,
            logger_factory,
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
    P: IndexerAssignmentProviderTrait,
{
    async fn create_indexer(
        &self,
        name: IndexerName,
        hash: DeploymentHash,
        node_id: NodeId,
    ) -> Result<CreateIndexerResponse, IndexerRegistrarError> {
        let id = self.store.create_indexer(name.clone())?;

        // We don't have a location for the subgraph yet; that will be
        // assigned when we deploy for real. For logging purposes, make up a
        // fake locator
        let logger = self
            .logger_factory
            .indexer_logger(&DeploymentLocator::new(DeploymentId(0), hash.clone()));

        let raw: serde_yaml::Mapping = {
            let file_bytes = self
                .resolver
                .cat(&logger, &hash.to_ipfs_link())
                .await
                .map_err(|e| {
                    IndexerRegistrarError::ResolveError(IndexerManifestResolveError::ResolveError(
                        e,
                    ))
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
                    &logger,
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

        let locations = self.store.locators(&hash)?;
        let deployment = match locations.len() {
            0 => return Err(IndexerRegistrarError::DeploymentNotFound(hash.to_string())),
            1 => locations[0].clone(),
            _ => {
                return Err(IndexerRegistrarError::StoreError(
                    anyhow!(
                        "there are {} different deployments with id {}",
                        locations.len(),
                        hash.as_str()
                    )
                    .into(),
                ))
            }
        };

        self.provider.start(deployment).await;

        Ok(CreateIndexerResponse { id })
    }
}

async fn create_indexer_version<C: Blockchain, S: IndexerStore, L: LinkResolver>(
    logger: &Logger,
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
        &logger,
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
