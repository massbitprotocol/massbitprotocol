pub use super::manifest::{
    DataSource, DataSourceTemplate, UnresolvedDataSource, UnresolvedDataSourceTemplate,
};
use super::types::{BlockHash, BlockPtr};
use crate::graph::prelude::BlockNumber;
use crate::graph::runtime::{AscHeap, AscPtr, DeterministicHostError, HostExportError};
use crate::prelude::{slog::SendSyncRefUnwindSafeKV, Logger};
use anyhow::Error;
use async_trait::async_trait;
use std::sync::Arc;
use std::{collections::BTreeMap, fmt::Debug};
use std::{collections::HashMap, convert::TryFrom};

use crate::mapping::HostFnCtx;
use thiserror::Error;
use web3::types::H256;

#[derive(Error, Debug)]
pub enum IngestorError {
    /// The Ethereum node does not know about this block for some reason, probably because it
    /// disappeared in a chain reorg.
    #[error("Block data unavailable, block was likely uncled (block hash = {0:?})")]
    BlockUnavailable(H256),

    /// An unexpected error occurred.
    #[error("Ingestor error: {0}")]
    Unknown(Error),
}
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

    /// Additional key-value pairs to be logged with the "Done processing trigger" message.
    fn logging_extras(&self) -> Box<dyn SendSyncRefUnwindSafeKV> {
        Box::new(slog::o! {})
    }
}
/*
pub trait NodeCapabilities<C: Blockchain> {
    fn from_data_sources(data_sources: &[C::DataSource]) -> Self;
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

    fn node_capabilities(&self) -> C::NodeCapabilities;
}
 */
/// Host fn that receives one u32 argument and returns an u32.
/// The name for an AS fuction is in the format `<namespace>.<function>`.
#[derive(Clone)]
pub struct HostFn {
    pub name: &'static str,
    pub func: Arc<dyn Send + Sync + Fn(HostFnCtx, u32) -> Result<u32, HostExportError>>,
}
pub trait RuntimeAdapter<C: Blockchain>: Send + Sync {
    fn host_fns(&self, ds: &C::DataSource) -> Result<Vec<HostFn>, Error>;
}
#[async_trait]
// This is only `Debug` because some tests require that
pub trait Blockchain: Debug + Sized + Send + Sync + 'static {
    // The `Clone` bound is used when reprocessing a block, because `triggers_in_block` requires an
    // owned `Block`. It would be good to come up with a way to remove this bound.
    //type Block: Block + Clone;
    type DataSource: DataSource<Self>;
    type UnresolvedDataSource: UnresolvedDataSource<Self>;

    type DataSourceTemplate: DataSourceTemplate<Self>;
    type UnresolvedDataSourceTemplate: UnresolvedDataSourceTemplate<Self>;

    //type TriggersAdapter: TriggersAdapter<Self>;

    /// Trigger data as parsed from the triggers adapter.
    type TriggerData: TriggerData + Ord;

    /// Decoded trigger ready to be processed by the mapping.
    type MappingTrigger: MappingTrigger + Debug;

    /// Trigger filter used as input to the triggers adapter.
    //type TriggerFilter: TriggerFilter<Self>;

    //type NodeCapabilities: NodeCapabilities<Self> + std::fmt::Display;

    //type IngestorAdapter: IngestorAdapter<Self>;

    //type RuntimeAdapter: RuntimeAdapter<Self>;

    fn reorg_threshold() -> u32;
    /*
       fn triggers_adapter(
           &self,
           loc: &DeploymentLocator,
           capabilities: &Self::NodeCapabilities,
           unified_api_version: UnifiedMappingApiVersion,
           stopwatch_metrics: StopwatchMetrics,ChainStore
       ) -> Result<Arc<Self::TriggersAdapter>, Error>;

       async fn new_block_stream(ChainStoreChainStoreChainStore
           &self,
           deployment: DeploymentLocator,
           start_blocks: Vec<BlockNumber>,
           filter: Self::TriggerFilter,
           metrics: Arc<BlockStreamMetrics>,
           unified_api_version: UnifiedMappingApiVersion,
       ) -> Result<BlockStream<Self>, Error>;

       fn ingestor_adapter(&self) -> Arc<Self::IngestorAdapter>;

       fn chain_store(&self) -> Arc<dyn ChainStore>;

       async fn block_pointer_from_number(
           &self,
           logger: &Logger,
           number: BlockNumber,
       ) -> Result<BlockPtr, IngestorError>;

       fn runtime_adapter(&self) -> Arc<Self::RuntimeAdapter>;
    */
}
