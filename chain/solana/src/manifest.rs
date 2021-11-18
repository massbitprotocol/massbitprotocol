use crate::Chain;
use anyhow::anyhow;
use async_trait::async_trait;
use futures03::{future::try_join3, stream::FuturesOrdered, TryStreamExt as _, TryStreamExt};
use massbit::blockchain::{Blockchain, UnresolvedDataSource, UnresolvedDataSourceTemplate};
use massbit::data::indexer::{BaseIndexerManifest, IndexerManifestResolveError, MIN_SPEC_VERSION};
use massbit::prelude::serde_yaml::Mapping;
use massbit::prelude::{serde_yaml, DeploymentHash, Deserialize, LinkResolver, Serialize};
use massbit::slog::Logger;
use semver::Version;
/// IndexerManifest with IPFS links unresolved
type UnresolvedSolanaIndexerManifest = BaseIndexerManifest<
    Chain,
    UnresolvedSolanaSchema,
    <Chain as Blockchain>::UnresolvedDataSource,
    <Chain as Blockchain>::UnresolvedDataSourceTemplate,
>;

pub type SolanaIndexerManifest = BaseIndexerManifest<
    Chain,
    SolanaSchema,
    <Chain as Blockchain>::DataSource,
    <Chain as Blockchain>::DataSourceTemplate,
>;

/// A validated and preprocessed GraphQL schema for a indexer.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct UnresolvedSolanaSchema {}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct SolanaSchema {}

#[async_trait]
pub trait ManifestResolve {
    async fn resolve_from_raw(
        logger: &Logger,
        id: DeploymentHash,
        mut raw: serde_yaml::Mapping,
        resolver: &impl LinkResolver,
        max_spec_version: semver::Version,
    ) -> Result<SolanaIndexerManifest, IndexerManifestResolveError>;
}
#[async_trait]
pub trait ManifestUnresolve {
    async fn resolve(
        self,
        resolver: &impl LinkResolver,
        logger: &Logger,
        max_spec_version: semver::Version,
    ) -> Result<SolanaIndexerManifest, anyhow::Error>;
}
impl UnresolvedSolanaSchema {
    async fn resolve(
        &self,
        id: DeploymentHash,
        resolver: &impl LinkResolver,
        logger: &Logger,
    ) -> Result<SolanaSchema, anyhow::Error> {
        //Empty SolanaSchema
        Ok(SolanaSchema::default())
    }
}
#[async_trait]
impl ManifestResolve for SolanaIndexerManifest {
    async fn resolve_from_raw(
        logger: &Logger,
        id: DeploymentHash,
        mut raw: Mapping,
        resolver: &impl LinkResolver,
        max_spec_version: Version,
    ) -> Result<SolanaIndexerManifest, IndexerManifestResolveError> {
        // Inject the IPFS hash as the ID of the indexer into the definition.
        raw.insert(
            serde_yaml::Value::from("id"),
            serde_yaml::Value::from(id.to_string()),
        );

        // Parse the YAML data into an UnresolvedIndexerManifest
        let unresolved: UnresolvedSolanaIndexerManifest = serde_yaml::from_value(raw.into())?;

        unresolved
            .resolve(&*resolver, logger, max_spec_version)
            .await
            .map_err(IndexerManifestResolveError::ResolveError)
    }
}

#[async_trait]
impl ManifestUnresolve for UnresolvedSolanaIndexerManifest {
    async fn resolve(
        self,
        resolver: &impl LinkResolver,
        logger: &Logger,
        max_spec_version: semver::Version,
    ) -> Result<SolanaIndexerManifest, anyhow::Error> {
        let UnresolvedSolanaIndexerManifest {
            id,
            spec_version,
            description,
            repository,
            schema,
            data_sources,
            templates,
            chain,
        } = self;

        if !(MIN_SPEC_VERSION..=max_spec_version.clone()).contains(&spec_version) {
            return Err(anyhow!(
                "This Graph Node only supports manifest spec versions between {} and {}, but indexer `{}` uses `{}`",
                MIN_SPEC_VERSION,
                max_spec_version,
                id,
                spec_version
            ));
        }

        let (schema, data_sources, templates) = try_join3(
            schema.resolve(id.clone(), resolver, logger),
            data_sources
                .into_iter()
                .map(|ds| ds.resolve(resolver, logger))
                .collect::<FuturesOrdered<_>>()
                .try_collect::<Vec<_>>(),
            templates
                .into_iter()
                .map(|template| template.resolve(resolver, logger))
                .collect::<FuturesOrdered<_>>()
                .try_collect::<Vec<_>>(),
        )
        .await?;

        Ok(SolanaIndexerManifest {
            id,
            spec_version,
            description,
            repository,
            schema,
            data_sources,
            templates,
            chain,
        })
    }
}
// impl<C: Blockchain> SolanaIndexerManifest<C> {
//     /// Entry point for resolving a indexer definition.
//     pub async fn resolve_from_raw(
//         logger: &Logger,
//         id: DeploymentHash,
//         mut raw: serde_yaml::Mapping,
//         resolver: &impl LinkResolver,
//         max_spec_version: semver::Version,
//     ) -> Result<Self, IndexerManifestResolveError> {
//         // Inject the IPFS hash as the ID of the indexer into the definition.
//         raw.insert(
//             serde_yaml::Value::from("id"),
//             serde_yaml::Value::from(id.to_string()),
//         );
//
//         // Parse the YAML data into an UnresolvedIndexerManifest
//         let unresolved: UnresolvedIndexerManifest<C> = serde_yaml::from_value(raw.into())?;
//
//         unresolved
//             .resolve(&*resolver, logger, max_spec_version)
//             .await
//             .map_err(IndexerManifestResolveError::ResolveError)
//     }
// }
