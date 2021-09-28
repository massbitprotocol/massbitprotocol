use anyhow::ensure;
use anyhow::{anyhow, Error};
use futures03::{future::try_join, stream::FuturesOrdered, TryStreamExt as _};
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
use crate::data::store::Entity;
use crate::prelude::*;

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
/// `Qm..` string that `graph-cli` prints when deploying to a subgraph
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
    /// Check that `s` is a valid `SubgraphDeploymentId` and create a new one.
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

        // Allow only deployment id's for 'real' subgraphs, not the old
        // metadata subgraph.
        if s == "subgraphs" {
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IndexerName(String);

impl IndexerName {
    pub fn new(s: impl Into<String>) -> Result<Self, ()> {
        let s = s.into();

        // Note: these validation rules must be kept consistent with the validation rules
        // implemented in any other components that rely on subgraph names.

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
            .map_err(|()| de::Error::invalid_value(de::Unexpected::Str(&s), &"valid subgraph name"))
    }
}

/// Result of a creating a subgraph in the registar.
#[derive(Serialize)]
pub struct CreateIndexerResponse {
    /// The ID of the subgraph that was created.
    pub id: String,
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
pub enum IndexerManifestResolveError {
    #[error("parse error: {0}")]
    ParseError(serde_yaml::Error),
    #[error("subgraph is not UTF-8")]
    NonUtf8,
    #[error("subgraph is not valid YAML")]
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
pub struct BaseIndexerManifest<C, D, T> {
    pub id: DeploymentHash,
    pub spec_version: Version,
    pub data_sources: Vec<D>,
    #[serde(default)]
    pub templates: Vec<T>,
    #[serde(skip_serializing, default)]
    pub chain: PhantomData<C>,
}

/// IndexerManifest with IPFS links unresolved
type UnresolvedIndexerManifest<C> = BaseIndexerManifest<
    C,
    <C as Blockchain>::UnresolvedDataSource,
    <C as Blockchain>::UnresolvedDataSourceTemplate,
>;

pub type IndexerManifest<C> =
    BaseIndexerManifest<C, <C as Blockchain>::DataSource, <C as Blockchain>::DataSourceTemplate>;

impl<C: Blockchain> IndexerManifest<C> {
    /// Entry point for resolving a indexer definition.
    pub async fn resolve_from_raw(
        id: DeploymentHash,
        mut raw: serde_yaml::Mapping,
        resolver: &impl LinkResolver,
    ) -> Result<Self, IndexerManifestResolveError> {
        // Inject the IPFS hash as the ID of the subgraph into the definition.
        raw.insert(
            serde_yaml::Value::from("id"),
            serde_yaml::Value::from(id.to_string()),
        );

        // Parse the YAML data into an UnresolvedSubgraphManifest
        let unresolved: UnresolvedIndexerManifest<C> = serde_yaml::from_value(raw.into())?;

        unresolved
            .resolve(&*resolver)
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
    ) -> Result<IndexerManifest<C>, anyhow::Error> {
        let UnresolvedIndexerManifest {
            id,
            spec_version,
            data_sources,
            templates,
            chain,
        } = self;

        let (data_sources, templates) = try_join(
            data_sources
                .into_iter()
                .map(|ds| ds.resolve(resolver))
                .collect::<FuturesOrdered<_>>()
                .try_collect::<Vec<_>>(),
            templates
                .into_iter()
                .map(|template| template.resolve(resolver))
                .collect::<FuturesOrdered<_>>()
                .try_collect::<Vec<_>>(),
        )
        .await?;

        Ok(IndexerManifest {
            id,
            spec_version,
            data_sources,
            templates,
            chain,
        })
    }
}
