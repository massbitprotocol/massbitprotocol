//! The `blockchain` module exports the necessary traits and data structures to integrate a
//! blockchain into Massbit. A blockchain is represented by an implementation of the `Blockchain`
//! trait which is the centerpiece of this module.

pub mod block_stream;
pub mod polling_block_stream;

mod types;

use anyhow::{anyhow, Context, Error};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::convert::TryFrom;
use std::sync::Arc;
use std::{
    fmt::{self, Debug},
    str::FromStr,
};

use crate::components::indexer::DataSourceTemplateInfo;
use crate::components::link_resolver::LinkResolver;
use crate::components::store::{BlockNumber, DeploymentLocator, StoredDynamicDataSource};
use crate::data::indexer::{DataSourceContext, IndexerManifestValidationError};
use crate::prelude::CheapClone;
use crate::runtime::{AscHeap, AscPtr, DeterministicHostError, HostExportError};

pub use block_stream::{BlockStream, TriggersAdapter};
pub use polling_block_stream::PollingBlockStream;
pub use types::{BlockHash, BlockPtr};

pub trait Block: Send + Sync {
    fn ptr(&self) -> BlockPtr;

    fn parent_ptr(&self) -> Option<BlockPtr>;

    fn number(&self) -> i32 {
        self.ptr().number
    }

    fn hash(&self) -> BlockHash {
        self.ptr().hash
    }

    fn parent_hash(&self) -> Option<BlockHash> {
        self.parent_ptr().map(|ptr| ptr.hash)
    }
}

#[async_trait]
pub trait Blockchain: Debug + Sized + Send + Sync + Unpin + 'static {
    const KIND: BlockchainKind;

    type Block: Block + Clone;

    type DataSource: DataSource<Self>;
    type UnresolvedDataSource: UnresolvedDataSource<Self>;

    type DataSourceTemplate: DataSourceTemplate<Self>;
    type UnresolvedDataSourceTemplate: UnresolvedDataSourceTemplate<Self>;

    type TriggersAdapter: TriggersAdapter<Self>;

    /// Trigger data as parsed from the triggers adapter.
    type TriggerData: TriggerData + Ord;

    /// Decoded trigger ready to be processed by the mapping.
    type MappingTrigger: MappingTrigger + Debug;

    /// Trigger filter used as input to the triggers adapter.
    type TriggerFilter: TriggerFilter<Self>;

    type RuntimeAdapter: RuntimeAdapter<Self>;

    fn triggers_adapter(&self) -> Result<Arc<Self::TriggersAdapter>, Error>;

    fn runtime_adapter(&self) -> Arc<Self::RuntimeAdapter>;

    async fn new_block_stream(
        &self,
        deployment: DeploymentLocator,
        start_block: BlockNumber,
        filter: Arc<Self::TriggerFilter>,
    ) -> Result<Box<dyn BlockStream<Self>>, Error>;

    async fn block_pointer_from_number(&self, number: BlockNumber) -> Result<BlockPtr, Error>;
}

pub trait TriggerFilter<C: Blockchain>: Default + Clone + Send + Sync {
    fn from_data_sources<'a>(
        data_sources: impl Iterator<Item = &'a C::DataSource> + Clone,
    ) -> Self {
        let mut this = Self::default();
        this.extend(data_sources);
        this
    }

    fn extend<'a>(&mut self, data_sources: impl Iterator<Item = &'a C::DataSource> + Clone);
}

pub trait DataSource<C: Blockchain>:
    'static + Sized + Send + Sync + Clone + TryFrom<DataSourceTemplateInfo<C>, Error = anyhow::Error>
{
    fn address(&self) -> Option<&[u8]>;
    fn start_block(&self) -> BlockNumber;
    fn name(&self) -> &str;
    fn kind(&self) -> &str;
    fn network(&self) -> Option<&str>;
    fn context(&self) -> Arc<Option<DataSourceContext>>;
    fn creation_block(&self) -> Option<BlockNumber>;
    fn api_version(&self) -> semver::Version;
    fn runtime(&self) -> &[u8];

    /// Checks if `trigger` matches this data source, and if so decodes it into a `MappingTrigger`.
    /// A return of `Ok(None)` mean the trigger does not match.
    fn match_and_decode(
        &self,
        trigger: &C::TriggerData,
        block: Arc<C::Block>,
    ) -> Result<Option<C::MappingTrigger>, Error>;

    fn is_duplicate_of(&self, other: &Self) -> bool;

    fn as_stored_dynamic_data_source(&self) -> StoredDynamicDataSource;

    fn from_stored_dynamic_data_source(
        templates: &BTreeMap<&str, &C::DataSourceTemplate>,
        stored: StoredDynamicDataSource,
    ) -> Result<Self, Error>;

    /// Used as part of manifest validation. If there are no errors, return an empty vector.
    fn validate(&self) -> Vec<IndexerManifestValidationError>;
}

#[async_trait]
pub trait UnresolvedDataSource<C: Blockchain>:
    'static + Sized + Send + Sync + DeserializeOwned
{
    async fn resolve(self, resolver: &impl LinkResolver) -> Result<C::DataSource, anyhow::Error>;
}

#[async_trait]
pub trait UnresolvedDataSourceTemplate<C: Blockchain>:
    'static + Sized + Send + Sync + DeserializeOwned + Default
{
    async fn resolve(
        self,
        resolver: &impl LinkResolver,
    ) -> Result<C::DataSourceTemplate, anyhow::Error>;
}

pub trait DataSourceTemplate<C: Blockchain>: Send + Sync + Clone + Debug {
    fn runtime(&self) -> &[u8];
    fn api_version(&self) -> semver::Version;
    fn name(&self) -> &str;
}

pub trait TriggerData {
    /// If there is an error when processing this trigger, this will called to add relevant context.
    /// For example an useful return is: `"block #<N> (<hash>), transaction <tx_hash>".
    fn error_context(&self) -> String;
}

pub trait MappingTrigger: Send + Sync {
    fn handler_name(&self) -> &str;

    /// A flexible interface for writing a type to AS memory, any pointer can be returned.
    /// Use `AscPtr::erased` to convert `AscPtr<T>` into `AscPtr<()>`.
    fn to_asc_ptr<H: AscHeap>(self, heap: &mut H) -> Result<AscPtr<()>, DeterministicHostError>;
}

pub struct HostFnCtx<'a> {
    pub block_ptr: BlockPtr,
    pub heap: &'a mut dyn AscHeap,
}

/// Host fn that receives one u32 argument and returns an u32.
/// The name for an AS fuction is in the format `<namespace>.<function>`.
#[derive(Clone)]
pub struct HostFn {
    pub name: &'static str,
    pub func: Arc<dyn Send + Sync + Fn(HostFnCtx, u32) -> Result<u32, HostExportError>>,
}

impl CheapClone for HostFn {
    fn cheap_clone(&self) -> Self {
        HostFn {
            name: self.name,
            func: self.func.cheap_clone(),
        }
    }
}

pub trait RuntimeAdapter<C: Blockchain>: Send + Sync {
    fn host_fns(&self, ds: &C::DataSource) -> Result<Vec<HostFn>, Error>;
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum BlockchainKind {
    /// Ethereum itself or chains that are compatible.
    Ethereum,
}

impl fmt::Display for BlockchainKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            BlockchainKind::Ethereum => "ethereum",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for BlockchainKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ethereum" => Ok(BlockchainKind::Ethereum),
            _ => Err(anyhow!("unknown blockchain kind {}", s)),
        }
    }
}

impl BlockchainKind {
    pub fn from_manifest(manifest: &serde_yaml::Mapping) -> Result<Self, Error> {
        use serde_yaml::Value;
        // The `kind` field of the first data source in the manifest.
        //
        // Split by `/` to, for example, read 'ethereum' in 'ethereum/contracts'.
        manifest
            .get(&Value::String("dataSources".to_owned()))
            .and_then(|ds| ds.as_sequence())
            .and_then(|ds| ds.first())
            .and_then(|ds| ds.as_mapping())
            .and_then(|ds| ds.get(&Value::String("kind".to_owned())))
            .and_then(|kind| kind.as_str())
            .and_then(|kind| kind.split('/').next())
            .context("invalid manifest")
            .and_then(BlockchainKind::from_str)
    }
}

/// A collection of blockchains, keyed by `BlockchainKind` and network.
#[derive(Default)]
pub struct BlockchainMap(HashMap<(BlockchainKind, String), Arc<dyn Any + Send + Sync>>);

impl BlockchainMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert<C: Blockchain>(&mut self, network: String, chain: Arc<C>) {
        self.0.insert((C::KIND, network), chain);
    }

    pub fn get<C: Blockchain>(&self, network: String) -> Result<Arc<C>, Error> {
        self.0
            .get(&(C::KIND, network.clone()))
            .with_context(|| format!("no network {} found on chain {}", network, C::KIND))?
            .cheap_clone()
            .downcast()
            .map_err(|_| anyhow!("unable to downcast, wrong type for blockchain {}", C::KIND))
    }
}
