use super::blockchain::Blockchain;
use crate::graph::cheap_clone::CheapClone;
use crate::graph::data::store::Entity;
use crate::graph::prelude::BlockNumber;
use crate::prelude::{Arc, Logger, Version};
use anyhow::{anyhow, ensure, Error};
use async_trait::async_trait;
use diesel::query_dsl::InternalJoinDsl;
//use ethabi::ethereum_types::H256;
use ethabi::{Address, Contract};

use crate::graph::util::ethereum::string_to_h256;
use futures03::future::try_join;
use futures03::stream::FuturesOrdered;
use futures03::Stream;
use futures03::TryStreamExt;
use itertools::Itertools;
use lazy_static::lazy_static;
use semver::VersionReq;
use serde::de;
use serde::de::DeserializeOwned;
use serde::ser;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use serde_yaml;
use slog::info;
use std::ops::Deref;
use std::pin::Pin;
use std::str::FromStr;
use std::time::Duration;
use std::{collections::BTreeMap, fmt::Debug};
use std::{collections::BTreeSet, marker::PhantomData};
use std::{collections::HashMap, convert::TryFrom};
use thiserror::Error;
use web3::types::H256;

pub const API_VERSION_0_0_5: Version = Version::new(0, 0, 5);
pub const API_VERSION_0_0_4: Version = Version::new(0, 0, 4);
lazy_static! {
    static ref MAX_API_VERSION: Version = std::env::var("MAX_API_VERSION")
        .ok()
        .and_then(|api_version_str| Version::parse(&api_version_str).ok())
        .unwrap_or(Version::new(0, 0, 4));
}
pub type DataSourceContext = Entity;
#[derive(Clone, Debug)]
pub struct DataSourceTemplateInfo<C: Blockchain> {
    pub template: C::DataSourceTemplate,
    pub params: Vec<String>,
    pub context: Option<DataSourceContext>,
    pub creation_block: BlockNumber,
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
/// The values that `json_stream` returns. The struct contains the deserialized
/// JSON value from the input stream, together with the line number from which
/// the value was read.
pub struct JsonStreamValue {
    pub value: Value,
    pub line: usize,
}

pub type JsonValueStream =
    Pin<Box<dyn Stream<Item = Result<JsonStreamValue, Error>> + Send + 'static>>;

/// Resolves links to subgraph manifests and resources referenced by them.
#[async_trait]
pub trait LinkResolver: Send + Sync + 'static {
    /// Updates the timeout used by the resolver.
    fn with_timeout(self, timeout: Duration) -> Self
    where
        Self: Sized;

    /// Enables infinite retries.
    fn with_retries(self) -> Self
    where
        Self: Sized;

    /// Fetches the link contents as bytes.
    async fn cat(&self, logger: &Logger, link: &Link) -> Result<Vec<u8>, Error>;

    /// Read the contents of `link` and deserialize them into a stream of JSON
    /// values. The values must each be on a single line; newlines are significant
    /// as they are used to split the file contents and each line is deserialized
    /// separately.
    async fn json_stream(&self, logger: &Logger, link: &Link) -> Result<JsonValueStream, Error>;
}
#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub struct UnresolvedSchema {
    pub file: Link,
}
pub struct StoredDynamicDataSource {
    pub name: String,
    pub source: Source,
    pub context: Option<String>,
    pub creation_block: Option<BlockNumber>,
}
pub trait DataSource<C: Blockchain>:
    'static + Sized + Send + Sync + Clone + TryFrom<DataSourceTemplateInfo<C>, Error = anyhow::Error>
{
    // ETHDEP: `Mapping` is Ethereum-specific.
    fn mapping(&self) -> &Mapping;

    fn address(&self) -> Option<&[u8]>;
    fn start_block(&self) -> BlockNumber;

    fn from_manifest(
        kind: String,
        network: Option<String>,
        name: String,
        source: Source,
        mapping: Mapping,
        context: Option<DataSourceContext>,
    ) -> Result<Self, Error>;

    fn name(&self) -> &str;
    fn kind(&self) -> &str;
    fn network(&self) -> Option<&str>;
    fn context(&self) -> Arc<Option<DataSourceContext>>;
    fn creation_block(&self) -> Option<BlockNumber>;
    /*
    /// Checks if `trigger` matches this data source, and if so decodes it into a `MappingTrigger`.
    /// A return of `Ok(None)` mean the trigger does not match.
    fn match_and_decode(
        &self,
        trigger: &C::TriggerData,
        block: Arc<C::Block>,
        logger: &Logger,
    ) -> Result<Option<C::MappingTrigger>, Error>;
    */
    fn is_duplicate_of(&self, other: &Self) -> bool;

    fn as_stored_dynamic_data_source(&self) -> StoredDynamicDataSource;

    fn from_stored_dynamic_data_source(
        templates: &BTreeMap<&str, &C::DataSourceTemplate>,
        stored: StoredDynamicDataSource,
    ) -> Result<Self, Error>;
}
#[async_trait]
pub trait UnresolvedDataSourceTemplate<C: Blockchain>:
    'static + Sized + Send + Sync + DeserializeOwned + Default
{
    async fn resolve(
        self,
        resolver: &impl LinkResolver,
        logger: &Logger,
    ) -> Result<C::DataSourceTemplate, anyhow::Error>;
}

pub trait DataSourceTemplate<C: Blockchain>: Send + Sync + Clone + Debug {
    fn mapping(&self) -> &Mapping;
    fn name(&self) -> &str;
}

#[async_trait]
pub trait UnresolvedDataSource<C: Blockchain>:
    'static + Sized + Send + Sync + DeserializeOwned
{
    async fn resolve(
        self,
        resolver: &impl LinkResolver,
        logger: &Logger,
    ) -> Result<C::DataSource, anyhow::Error>;
}
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

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq, Deserialize)]
pub struct TemplateSource {
    pub abi: String,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub struct UnresolvedMappingABI {
    pub name: String,
    pub file: Link,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MappingABI {
    pub name: String,
    pub contract: Contract,
}

impl UnresolvedMappingABI {
    pub async fn resolve(
        self,
        resolver: &impl LinkResolver,
        logger: &Logger,
    ) -> Result<MappingABI, anyhow::Error> {
        info!(
            logger,
            "Resolve ABI";
            "name" => &self.name,
            "link" => &self.file.link
        );

        let contract_bytes = resolver.cat(&logger, &self.file).await?;
        let contract = Contract::load(&*contract_bytes)?;
        Ok(MappingABI {
            name: self.name,
            contract,
        })
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub struct MappingBlockHandler {
    pub handler: String,
    pub filter: Option<BlockHandlerFilter>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum BlockHandlerFilter {
    // Call filter will trigger on all blocks where the data source contract
    // address has been called
    Call,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub struct MappingCallHandler {
    pub function: String,
    pub handler: String,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub struct MappingEventHandler {
    pub event: String,
    pub topic0: Option<H256>,
    pub handler: String,
}

impl MappingEventHandler {
    pub fn topic0(&self) -> H256 {
        self.topic0
            .unwrap_or_else(|| string_to_h256(&self.event.replace("indexed ", "")))
    }
}

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnresolvedMapping {
    pub kind: String,
    pub api_version: String,
    pub language: String,
    pub entities: Vec<String>,
    pub abis: Vec<UnresolvedMappingABI>,
    #[serde(default)]
    pub block_handlers: Vec<MappingBlockHandler>,
    #[serde(default)]
    pub call_handlers: Vec<MappingCallHandler>,
    #[serde(default)]
    pub event_handlers: Vec<MappingEventHandler>,
    pub file: Link,
}

#[derive(Clone, Debug)]
pub struct Mapping {
    pub kind: String,
    pub api_version: Version,
    pub language: String,
    pub entities: Vec<String>,
    pub abis: Vec<Arc<MappingABI>>,
    pub block_handlers: Vec<MappingBlockHandler>,
    pub call_handlers: Vec<MappingCallHandler>,
    pub event_handlers: Vec<MappingEventHandler>,
    pub runtime: Arc<Vec<u8>>,
    //pub link: Link,
}

impl Mapping {
    pub fn requires_archive(&self) -> bool {
        self.calls_host_fn("ethereum.call")
    }

    fn calls_host_fn(&self, host_fn: &str) -> bool {
        use wasmparser::Payload;

        let runtime = self.runtime.as_ref().as_ref();

        for payload in wasmparser::Parser::new(0).parse_all(runtime) {
            match payload.unwrap() {
                Payload::ImportSection(s) => {
                    for import in s {
                        let import = import.unwrap();
                        if import.field == Some(host_fn) {
                            return true;
                        }
                    }
                }
                _ => (),
            }
        }

        false
    }

    pub fn has_call_handler(&self) -> bool {
        !self.call_handlers.is_empty()
    }

    pub fn has_block_handler_with_call_filter(&self) -> bool {
        self.block_handlers
            .iter()
            .any(|handler| matches!(handler.filter, Some(BlockHandlerFilter::Call)))
    }

    pub fn find_abi(&self, abi_name: &str) -> Result<Arc<MappingABI>, Error> {
        Ok(self
            .abis
            .iter()
            .find(|abi| abi.name == abi_name)
            .ok_or_else(|| anyhow!("No ABI entry with name `{}` found", abi_name))?
            .cheap_clone())
    }
}

impl UnresolvedMapping {
    pub async fn resolve(
        self,
        resolver: &impl LinkResolver,
        logger: &Logger,
    ) -> Result<Mapping, anyhow::Error> {
        let UnresolvedMapping {
            kind,
            api_version,
            language,
            entities,
            abis,
            block_handlers,
            call_handlers,
            event_handlers,
            file: link,
        } = self;

        let api_version = Version::parse(&api_version)?;

        ensure!(
            VersionReq::parse(&format!("<= {}", *MAX_API_VERSION))
                .unwrap()
                .matches(&api_version),
            "The maximum supported mapping API version of this indexer is {}, but `{}` was found",
            *MAX_API_VERSION,
            api_version
        );

        info!(logger, "Resolve mapping"; "link" => &link.link);

        let (abis, runtime) = try_join(
            // resolve each abi
            abis.into_iter()
                .map(|unresolved_abi| async {
                    Result::<_, Error>::Ok(Arc::new(
                        unresolved_abi.resolve(resolver, logger).await?,
                    ))
                })
                .collect::<FuturesOrdered<_>>()
                .try_collect::<Vec<_>>(),
            async {
                let module_bytes = resolver.cat(logger, &link).await?;
                Ok(Arc::new(module_bytes))
            },
        )
        .await?;

        Ok(Mapping {
            kind,
            api_version,
            language,
            entities,
            abis,
            block_handlers: block_handlers.clone(),
            call_handlers: call_handlers.clone(),
            event_handlers: event_handlers.clone(),
            runtime,
            //link,
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct UnifiedMappingApiVersion(Option<Version>);

impl UnifiedMappingApiVersion {
    pub fn equal_or_greater_than(&self, other_version: &Version) -> bool {
        assert!(
            other_version >= &API_VERSION_0_0_5,
            "api versions before 0.0.5 should not be used for comparison"
        );
        match &self.0 {
            Some(version) => version >= other_version,
            None => false,
        }
    }

    pub fn try_from_versions<'a>(
        versions: impl Iterator<Item = &'a Version>,
    ) -> Result<Self, DifferentMappingApiVersions> {
        let unique_versions: BTreeSet<Version> = versions.into_iter().cloned().collect();

        let all_below_referential_version = unique_versions.iter().all(|v| *v < API_VERSION_0_0_5);
        let all_the_same = unique_versions.len() == 1;

        let unified_version: Option<Version> = match (all_below_referential_version, all_the_same) {
            (false, false) => return Err(unique_versions.into()),
            (false, true) => Some(unique_versions.iter().nth(0).unwrap().deref().clone()),
            (true, _) => None,
        };

        Ok(UnifiedMappingApiVersion(unified_version))
    }
}

#[derive(Error, Debug, PartialEq)]
#[error("Expected a single apiVersion for mappings. Found: {}.", format_versions(.0))]
pub struct DifferentMappingApiVersions(BTreeSet<Version>);

fn format_versions(versions: &BTreeSet<Version>) -> String {
    versions.iter().map(ToString::to_string).join(", ")
}

impl From<BTreeSet<Version>> for DifferentMappingApiVersions {
    fn from(versions: BTreeSet<Version>) -> Self {
        Self(versions)
    }
}
