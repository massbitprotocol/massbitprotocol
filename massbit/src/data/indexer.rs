use anyhow::Error;
use futures03::{stream::FuturesOrdered, TryStreamExt as _};
use serde::de;
use std::marker::PhantomData;
use std::str::FromStr;
use thiserror::Error;
use web3::types::Address;

use crate::blockchain::{Blockchain, DataSource, UnresolvedDataSource as _};
use crate::components::link_resolver::LinkResolver;
use crate::prelude::{BlockNumber, Deserialize, Serialize};

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseIndexerManifest<C, D> {
    pub data_sources: Vec<D>,
    #[serde(skip_serializing, default)]
    pub chain: PhantomData<C>,
}

/// IndexerManifest with IPFS links unresolved
type UnresolvedIndexerManifest<C> = BaseIndexerManifest<C, <C as Blockchain>::UnresolvedDataSource>;

pub type IndexerManifest<C> = BaseIndexerManifest<C, <C as Blockchain>::DataSource>;

impl<C: Blockchain> IndexerManifest<C> {
    /// Entry point for resolving a indexer definition.
    pub async fn resolve_from_raw(
        id: &str,
        mut raw: serde_yaml::Mapping,
        resolver: &impl LinkResolver,
    ) -> Result<Self, IndexerManifestResolveError> {
        // Inject the IPFS hash as the ID of the indexer into the definition.
        raw.insert(
            serde_yaml::Value::from("id"),
            serde_yaml::Value::from(id.to_string()),
        );

        // Parse the YAML data into an UnresolvedIndexerManifest
        let unresolved: UnresolvedIndexerManifest<C> = serde_yaml::from_value(raw.into())?;
        unresolved
            .resolve(&*resolver)
            .await
            .map_err(IndexerManifestResolveError::ResolveError)
    }

    pub fn start_blocks(&self) -> Vec<BlockNumber> {
        self.data_sources
            .iter()
            .map(|data_source| data_source.start_block())
            .collect()
    }
}

impl<C: Blockchain> UnresolvedIndexerManifest<C> {
    pub async fn resolve(self, resolver: &impl LinkResolver) -> Result<IndexerManifest<C>, Error> {
        let UnresolvedIndexerManifest {
            data_sources,
            chain,
        } = self;
        let data_sources = data_sources
            .into_iter()
            .map(|ds| ds.resolve(resolver))
            .collect::<FuturesOrdered<_>>()
            .try_collect::<Vec<_>>()
            .await?;
        Ok(IndexerManifest {
            data_sources,
            chain,
        })
    }
}
