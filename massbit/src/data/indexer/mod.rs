use anyhow::ensure;
use anyhow::{anyhow, Error};
use diesel::helper_types::max;
use futures03::{future::try_join3, stream::FuturesOrdered, TryStreamExt as _};
use itertools::Itertools;
use lazy_static::lazy_static;
use semver::Version;
use serde::de;
use serde::ser;
use serde_yaml;
use stable_hash::prelude::*;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use std::{collections::BTreeSet, marker::PhantomData};
use thiserror::Error;
use wasmparser;
use web3::types::Address;

use crate::blockchain::{
    Blockchain, DataSource, DataSourceTemplate as _, UnresolvedDataSource as _,
    UnresolvedDataSourceTemplate as _,
};
use crate::components::link_resolver::LinkResolver;
use crate::components::store::DeploymentLocator;
use crate::data::graphql::TryFromValue;
use crate::data::query::QueryExecutionError;
use crate::data::schema::{SchemaImportError, SchemaValidationError};
use crate::data::store::Entity;
use crate::prelude::*;

lazy_static! {
    static ref DISABLE_GRAFTS: bool = std::env::var("GRAPH_DISABLE_GRAFTS")
        .ok()
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    pub static ref MAX_SPEC_VERSION: Version = std::env::var("GRAPH_MAX_SPEC_VERSION")
        .ok()
        .and_then(|api_version_str| Version::parse(&api_version_str).ok())
        .unwrap_or(SPEC_VERSION_0_0_3);
    static ref MAX_API_VERSION: semver::Version = std::env::var("GRAPH_MAX_API_VERSION")
        .ok()
        .and_then(|api_version_str| semver::Version::parse(&api_version_str).ok())
        .unwrap_or(semver::Version::new(0, 0, 5));
}

pub mod schema;
pub mod status;

/// This version adds a new indexer validation step that rejects manifests whose mappings have
/// different API versions if at least one of them is equal to or higher than `0.0.5`.
pub const API_VERSION_0_0_5: Version = Version::new(0, 0, 5);

/// Before this check was introduced, there were already indexers in the wild with spec version
/// 0.0.3, due to confusion with the api version. To avoid breaking those, we accept 0.0.3 though it
/// doesn't exist. In the future we should not use 0.0.3 as version and skip to 0.0.4 to avoid
/// ambiguity.
pub const SPEC_VERSION_0_0_3: Version = Version::new(0, 0, 3);

/// This version supports indexer feature management.
pub const SPEC_VERSION_0_0_4: Version = Version::new(0, 0, 4);

pub const MIN_SPEC_VERSION: Version = Version::new(0, 0, 2);

/// Deserialize an Address (with or without '0x' prefix).
fn deserialize_address<'de, D>(deserializer: D) -> Result<Option<Address>, D::Error>
where
    D: de::Deserializer<'de>,
{
    use serde::de::Error;

    let s: String = de::Deserialize::deserialize(deserializer)?;
    let address = s.trim_start_matches("0x");
    Address::from_str(address)
        .map_err(D::Error::custom)
        .map(Some)
}

// Note: This has a StableHash impl. Do not modify fields without a backward
// compatible change to the StableHash impl (below)
/// The IPFS hash used to identifiy a deployment externally, i.e., the
/// `Qm..` string that `graph-cli` prints when deploying to a indexer
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeploymentHash(String);

impl StableHash for DeploymentHash {
    #[inline]
    fn stable_hash<H: StableHasher>(&self, mut sequence_number: H::Seq, state: &mut H) {
        self.0.stable_hash(sequence_number.next_child(), state);
    }
}

/// `DeploymentHash` is fixed-length so cheap to clone.
impl CheapClone for DeploymentHash {}

impl DeploymentHash {
    /// Check that `s` is a valid `IndexerDeploymentId` and create a new one.
    /// If `s` is longer than 46 characters, or contains characters other than
    /// alphanumeric characters or `_`, return s (as a `String`) as the error
    pub fn new(s: impl Into<String>) -> Result<Self, String> {
        let s = s.into();

        // Enforce length limit
        if s.len() > 46 {
            return Err(s);
        }

        // Check that the ID contains only allowed characters.
        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(s);
        }

        // Allow only deployment id's for 'real' indexer, not the old
        // metadata indexer.
        if s == "indexer" {
            return Err(s);
        }

        Ok(DeploymentHash(s))
    }

    pub fn to_ipfs_link(&self) -> Link {
        Link {
            link: format!("/ipfs/{}", self),
        }
    }
}

impl Deref for DeploymentHash {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for DeploymentHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl ser::Serialize for DeploymentHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> de::Deserialize<'de> for DeploymentHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s: String = de::Deserialize::deserialize(deserializer)?;
        DeploymentHash::new(s)
            .map_err(|s| de::Error::invalid_value(de::Unexpected::Str(&s), &"valid subgraph name"))
    }
}

impl TryFromValue for DeploymentHash {
    fn try_from_value(value: &q::Value) -> Result<Self, Error> {
        Self::new(String::try_from_value(value)?)
            .map_err(|s| anyhow!("Invalid subgraph ID `{}`", s))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IndexerName(String);

impl IndexerName {
    pub fn new(s: impl Into<String>) -> Result<Self, ()> {
        let s = s.into();

        // Note: these validation rules must be kept consistent with the validation rules
        // implemented in any other components that rely on indexer names.

        // Enforce length limits
        if s.is_empty() || s.len() > 255 {
            return Err(());
        }

        // Check that the name contains only allowed characters.
        if !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/')
        {
            return Err(());
        }

        // Parse into components and validate each
        for part in s.split('/') {
            // Each part must be non-empty and not too long
            if part.is_empty() || part.len() > 32 {
                return Err(());
            }

            // To keep URLs unambiguous, reserve the token "graphql"
            if part == "graphql" {
                return Err(());
            }

            // Part should not start or end with a special character.
            let first_char = part.chars().next().unwrap();
            let last_char = part.chars().last().unwrap();
            if !first_char.is_ascii_alphanumeric()
                || !last_char.is_ascii_alphanumeric()
                || !part.chars().any(|c| c.is_ascii_alphabetic())
            {
                return Err(());
            }
        }

        Ok(IndexerName(s))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for IndexerName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl ser::Serialize for IndexerName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> de::Deserialize<'de> for IndexerName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s: String = de::Deserialize::deserialize(deserializer)?;
        IndexerName::new(s.clone())
            .map_err(|()| de::Error::invalid_value(de::Unexpected::Str(&s), &"valid indexer name"))
    }
}

/// Result of a creating a indexer in the registar.
#[derive(Serialize)]
pub struct CreateIndexerResponse {
    /// The ID of the indexer that was created.
    pub id: String,
}

#[derive(Error, Debug)]
pub enum IndexerRegistrarError {
    #[error("indexer resolve error: {0}")]
    ResolveError(IndexerManifestResolveError),
    #[error("indexer already exists: {0}")]
    NameExists(String),
    #[error("indexer name not found: {0}")]
    NameNotFound(String),
    #[error("Ethereum network not supported by registrar: {0}")]
    NetworkNotSupported(Error),
    #[error("deployment not found: {0}")]
    DeploymentNotFound(String),
    #[error("deployment assignment unchanged: {0}")]
    DeploymentAssignmentUnchanged(String),
    #[error("indexer registrar internal query error: {0}")]
    QueryExecutionError(QueryExecutionError),
    #[error("indexer registrar error with store: {0}")]
    StoreError(StoreError),
    #[error("indexer validation error: {}", display_vector(.0))]
    ManifestValidationError(Vec<IndexerManifestValidationError>),
    #[error("indexer deployment error: {0}")]
    IndexerDeploymentError(StoreError),
    #[error("indexer registrar error: {0}")]
    Unknown(anyhow::Error),
}

impl From<QueryExecutionError> for IndexerRegistrarError {
    fn from(e: QueryExecutionError) -> Self {
        IndexerRegistrarError::QueryExecutionError(e)
    }
}

impl From<StoreError> for IndexerRegistrarError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::DeploymentNotFound(id) => IndexerRegistrarError::DeploymentNotFound(id),
            e => IndexerRegistrarError::StoreError(e),
        }
    }
}

impl From<Error> for IndexerRegistrarError {
    fn from(e: Error) -> Self {
        IndexerRegistrarError::Unknown(e)
    }
}

impl From<IndexerManifestValidationError> for IndexerRegistrarError {
    fn from(e: IndexerManifestValidationError) -> Self {
        IndexerRegistrarError::ManifestValidationError(vec![e])
    }
}

#[derive(Error, Debug)]
pub enum IndexerAssignmentProviderError {
    #[error("Indexer resolve error: {0}")]
    ResolveError(Error),
    /// Occurs when attempting to remove a indexer that's not hosted.
    #[error("Indexer with ID {0} already running")]
    AlreadyRunning(DeploymentHash),
    #[error("Indexer with ID {0} is not running")]
    NotRunning(DeploymentLocator),
    #[error("Indexer provider error: {0}")]
    Unknown(anyhow::Error),
}

impl From<Error> for IndexerAssignmentProviderError {
    fn from(e: Error) -> Self {
        IndexerAssignmentProviderError::Unknown(e)
    }
}

impl From<::diesel::result::Error> for IndexerAssignmentProviderError {
    fn from(e: ::diesel::result::Error) -> Self {
        IndexerAssignmentProviderError::Unknown(e.into())
    }
}

#[derive(Error, Debug)]
pub enum IndexerManifestValidationError {
    #[error("indexer has no data sources")]
    NoDataSources,
    #[error("indexer source address is required")]
    SourceAddressRequired,
    #[error("indexer cannot index data from different Ethereum networks")]
    MultipleEthereumNetworks,
    #[error("indexer must have at least one Ethereum network data source")]
    EthereumNetworkRequired,
    #[error("indexer data source has too many similar block handlers")]
    DataSourceBlockHandlerLimitExceeded,
    #[error("the specified block must exist on the Ethereum network")]
    BlockNotFound(String),
    #[error("imported schema(s) are invalid: {0:?}")]
    SchemaImportError(Vec<SchemaImportError>),
    #[error("schema validation failed: {0:?}")]
    SchemaValidationError(Vec<SchemaValidationError>),
    #[error("the graft base is invalid: {0}")]
    GraftBaseInvalid(String),
    #[error("indexer must use a single apiVersion across its data sources. Found: {}", format_versions(.0))]
    DifferentApiVersions(BTreeSet<Version>),
}

#[derive(Error, Debug)]
pub enum IndexerManifestResolveError {
    #[error("parse error: {0}")]
    ParseError(serde_yaml::Error),
    #[error("indexer is not UTF-8")]
    NonUtf8,
    #[error("indexer is not valid YAML")]
    InvalidFormat,
    #[error("resolve error: {0}")]
    ResolveError(anyhow::Error),
}

impl From<serde_yaml::Error> for IndexerManifestResolveError {
    fn from(e: serde_yaml::Error) -> Self {
        IndexerManifestResolveError::ParseError(e)
    }
}

/// Data source contexts are conveniently represented as entities.
pub type DataSourceContext = Entity;

/// IPLD link.
#[derive(Clone, Debug, Default, Hash, Eq, PartialEq, Deserialize)]
pub struct Link {
    #[serde(rename = "/")]
    pub link: String,
}

impl From<String> for Link {
    fn from(s: String) -> Self {
        Self { link: s }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub struct UnresolvedSchema {
    pub file: Link,
}

impl UnresolvedSchema {
    pub async fn resolve(
        self,
        id: DeploymentHash,
        resolver: &impl LinkResolver,
        logger: &Logger,
    ) -> Result<Schema, anyhow::Error> {
        info!(logger, "Resolve schema"; "link" => &self.file.link);
        let schema_bytes = resolver.cat(logger, &self.file).await?;
        Schema::parse(&String::from_utf8(schema_bytes)?, id)
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub struct Source {
    /// The contract address for the data source. We allow data sources
    /// without an address for 'wildcard' triggers that catch all possible
    /// events with the given `abi`
    #[serde(default, deserialize_with = "deserialize_address")]
    pub address: Option<Address>,
    pub abi: String,
    #[serde(rename = "startBlock", default)]
    pub start_block: BlockNumber,
}

pub fn calls_host_fn(runtime: &[u8], host_fn: &str) -> anyhow::Result<bool> {
    use wasmparser::Payload;

    for payload in wasmparser::Parser::new(0).parse_all(runtime) {
        if let Payload::ImportSection(s) = payload? {
            for import in s {
                let import = import?;
                if import.field == Some(host_fn) {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseIndexerManifest<C, S, D, T> {
    pub id: DeploymentHash,
    pub spec_version: Version,
    pub description: Option<String>,
    pub repository: Option<String>,
    pub data_sources: Vec<D>,
    pub schema: S,
    #[serde(default)]
    pub templates: Vec<T>,
    #[serde(skip_serializing, default)]
    pub chain: PhantomData<C>,
}

/// IndexerManifest with IPFS links unresolved
type UnresolvedIndexerManifest<C> = BaseIndexerManifest<
    C,
    UnresolvedSchema,
    <C as Blockchain>::UnresolvedDataSource,
    <C as Blockchain>::UnresolvedDataSourceTemplate,
>;

pub type IndexerManifest<C> = BaseIndexerManifest<
    C,
    Schema,
    <C as Blockchain>::DataSource,
    <C as Blockchain>::DataSourceTemplate,
>;

/// Unvalidated IndexerManifest
pub struct UnvalidatedIndexerManifest<C: Blockchain>(IndexerManifest<C>);

impl<C: Blockchain> UnvalidatedIndexerManifest<C> {
    /// Entry point for resolving a indexer definition.
    /// Right now the only supported links are of the form:
    /// `/ipfs/QmUmg7BZC1YP1ca66rRtWKxpXp77WgVHrnv263JtDuvs2k`
    pub async fn resolve(
        id: DeploymentHash,
        raw: serde_yaml::Mapping,
        resolver: Arc<impl LinkResolver>,
        logger: &Logger,
        max_spec_version: semver::Version,
    ) -> Result<Self, IndexerManifestResolveError> {
        Ok(Self(
            IndexerManifest::resolve_from_raw(id, raw, resolver.deref(), logger, max_spec_version)
                .await?,
        ))
    }

    /// Validates the indexer manifest file.
    ///
    /// Graft base validation will be skipped if the parameter `validate_graft_base` is false.
    pub fn validate<S: IndexerStore>(
        self,
        store: Arc<S>,
    ) -> Result<IndexerManifest<C>, Vec<IndexerManifestValidationError>> {
        let (schemas, _) = self.0.schema.resolve_schema_references(store.clone());

        let mut errors: Vec<IndexerManifestValidationError> = vec![];

        // Validate that the manifest has at least one data source
        if self.0.data_sources.is_empty() {
            errors.push(IndexerManifestValidationError::NoDataSources);
        }

        for ds in &self.0.data_sources {
            errors.extend(ds.validate());
        }

        let mut networks = self
            .0
            .data_sources
            .iter()
            .filter(|d| d.kind().eq("ethereum/contract"))
            .filter_map(|d| d.network().map(|n| n.to_string()))
            .collect::<Vec<String>>();
        networks.sort();
        networks.dedup();
        match networks.len() {
            0 => errors.push(IndexerManifestValidationError::EthereumNetworkRequired),
            1 => (),
            _ => errors.push(IndexerManifestValidationError::MultipleEthereumNetworks),
        }

        self.0
            .schema
            .validate(&schemas)
            .err()
            .into_iter()
            .for_each(|schema_errors| {
                errors.push(IndexerManifestValidationError::SchemaValidationError(
                    schema_errors,
                ));
            });

        match errors.is_empty() {
            true => Ok(self.0),
            false => Err(errors),
        }
    }

    pub fn spec_version(&self) -> &Version {
        &self.0.spec_version
    }
}

impl<C: Blockchain> IndexerManifest<C> {
    /// Entry point for resolving a indexer definition.
    pub async fn resolve_from_raw(
        id: DeploymentHash,
        mut raw: serde_yaml::Mapping,
        resolver: &impl LinkResolver,
        logger: &Logger,
        max_spec_version: semver::Version,
    ) -> Result<Self, IndexerManifestResolveError> {
        // Inject the IPFS hash as the ID of the indexer into the definition.
        raw.insert(
            serde_yaml::Value::from("id"),
            serde_yaml::Value::from(id.to_string()),
        );

        // Parse the YAML data into an UnresolvedIndexerManifest
        let unresolved: UnresolvedIndexerManifest<C> = serde_yaml::from_value(raw.into())?;

        unresolved
            .resolve(&*resolver, logger, max_spec_version)
            .await
            .map_err(IndexerManifestResolveError::ResolveError)
    }

    pub fn network_name(&self) -> String {
        // Assume the manifest has been validated, ensuring network names are homogenous
        self.data_sources
            .iter()
            .filter(|d| d.kind() == "ethereum/contract")
            .filter_map(|d| d.network().map(|n| n.to_string()))
            .next()
            .expect("Validated manifest does not have a network defined on any datasource")
    }

    pub fn start_blocks(&self) -> Vec<BlockNumber> {
        self.data_sources
            .iter()
            .map(|data_source| data_source.start_block())
            .collect()
    }

    pub fn runtimes(&self) -> impl Iterator<Item = &[u8]> + '_ {
        self.templates
            .iter()
            .map(|template| template.runtime())
            .chain(self.data_sources.iter().map(|source| source.runtime()))
    }
}

impl<C: Blockchain> UnresolvedIndexerManifest<C> {
    pub async fn resolve(
        self,
        resolver: &impl LinkResolver,
        logger: &Logger,
        max_spec_version: semver::Version,
    ) -> Result<IndexerManifest<C>, anyhow::Error> {
        let UnresolvedIndexerManifest {
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

        Ok(IndexerManifest {
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

fn display_vector(input: &Vec<impl std::fmt::Display>) -> impl std::fmt::Display {
    let formatted_errors = input
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join("; ");
    format!("[{}]", formatted_errors)
}

fn format_versions(versions: &BTreeSet<Version>) -> String {
    versions.iter().map(ToString::to_string).join(", ")
}
