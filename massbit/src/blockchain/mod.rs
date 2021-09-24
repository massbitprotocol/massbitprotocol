//! The `blockchain` module exports the necessary traits and data structures to integrate a
//! blockchain into Massbit. A blockchain is represented by an implementation of the `Blockchain`
//! trait which is the centerpiece of this module.

pub mod block_stream;
pub mod polling_block_stream;

mod types;

use anyhow::{anyhow, Context, Error};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::{
    fmt::{self, Debug},
    str::FromStr,
};

use crate::components::link_resolver::LinkResolver;
use crate::components::store::BlockNumber;

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

    type Block: Block;
    type DataSource: DataSource<Self>;
    type UnresolvedDataSource: UnresolvedDataSource<Self>;

    type TriggersAdapter: TriggersAdapter<Self>;

    /// Trigger data as parsed from the triggers adapter.
    type TriggerData: TriggerData + Ord;

    /// Trigger filter used as input to the triggers adapter.
    type TriggerFilter: TriggerFilter<Self>;

    fn triggers_adapter(&self) -> Result<Arc<Self::TriggersAdapter>, Error>;

    async fn new_block_stream(
        &self,
        start_block: BlockNumber,
        filter: Arc<Self::TriggerFilter>,
    ) -> Result<Box<dyn BlockStream<Self>>, Error>;
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

pub trait DataSource<C: Blockchain>: 'static + Sized + Send + Sync + Clone {
    fn start_block(&self) -> BlockNumber;
}

#[async_trait]
pub trait UnresolvedDataSource<C: Blockchain>:
    'static + Sized + Send + Sync + DeserializeOwned
{
    async fn resolve(self, resolver: &impl LinkResolver) -> Result<C::DataSource, anyhow::Error>;
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

pub trait TriggerData {
    /// If there is an error when processing this trigger, this will called to add relevant context.
    /// For example an useful return is: `"block #<N> (<hash>), transaction <tx_hash>".
    fn error_context(&self) -> String;
}
