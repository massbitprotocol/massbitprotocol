//pub mod adapter;
pub mod data_source;
//pub mod network;
pub mod runtime;
pub mod trigger;
pub mod types;
use crate::chain::ethereum::data_source::{UnresolvedDataSource, UnresolvedDataSourceTemplate};
use crate::graph::prelude::MetricsRegistry;
use crate::graph::prelude::{BlockNumber, LoggerFactory};
use crate::indexer::blockchain::{Blockchain, IngestorError};
use crate::indexer::types::BlockPtr;
use crate::prelude::{Arc, Logger};
use async_trait::async_trait;
use data_source::{DataSource, DataSourceTemplate};
use types::LightEthereumBlock;

pub struct Chain {
    logger_factory: LoggerFactory,
    name: String,
    registry: Arc<dyn MetricsRegistry>,
    ancestor_count: BlockNumber,
    reorg_threshold: BlockNumber,
    is_ingestible: bool,
}
impl std::fmt::Debug for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "chain: ethereum")
    }
}

impl Chain {
    pub fn new(
        logger_factory: LoggerFactory,
        name: String,
        //node_id: NodeId,
        registry: Arc<dyn MetricsRegistry>,
        //chain_store: Arc<dyn ChainStore>,
        //call_cache: Arc<dyn EthereumCallCache>,
        //subgraph_store: Arc<dyn SubgraphStore>,
        //eth_adapters: EthereumNetworkAdapters,
        //chain_head_update_listener: Arc<dyn ChainHeadUpdateListener>,
        ancestor_count: BlockNumber,
        reorg_threshold: BlockNumber,
        is_ingestible: bool,
    ) -> Self {
        Chain {
            logger_factory,
            name,
            //node_id,
            registry,
            //eth_adapters: Arc::new(eth_adapters),

            //chain_store,
            //call_cache,
            //subgraph_store,
            //chain_head_update_listener,
            ancestor_count,
            reorg_threshold,
            is_ingestible,
        }
    }
}

#[async_trait]
impl Blockchain for Chain {
    //type Block = BlockFinality;
    type DataSource = DataSource;
    type UnresolvedDataSource = UnresolvedDataSource;
    type DataSourceTemplate = DataSourceTemplate;
    type UnresolvedDataSourceTemplate = UnresolvedDataSourceTemplate;
    type TriggerData = trigger::EthereumTrigger;

    type MappingTrigger = trigger::MappingTrigger;
    //type TriggerFilter = ();
    //type NodeCapabilities = ();
    //type RuntimeAdapter = ();

    fn reorg_threshold() -> u32 {
        todo!()
    }
    /*
    async fn block_pointer_from_number(
        &self,
        logger: &Logger,
        number: BlockNumber,
    ) -> Result<BlockPtr, IngestorError> {
        todo!()
    }

    fn runtime_adapter(&self) -> Arc<Self::RuntimeAdapter> {
        todo!()
    }
     */
}
/*
/// This is used in `EthereumAdapter::triggers_in_block`, called when re-processing a block for
/// newly created data sources. This allows the re-processing to be reorg safe without having to
/// always fetch the full block data.
#[derive(Clone, Debug)]
pub enum BlockFinality {
    /// If a block is final, we only need the header and the triggers.
    Final(Arc<LightEthereumBlock>),

    // If a block may still be reorged, we need to work with more local data.
    NonFinal(EthereumBlockWithCalls),
}

impl BlockFinality {
    pub(crate) fn light_block(&self) -> Arc<LightEthereumBlock> {
        match self {
            BlockFinality::Final(block) => block.cheap_clone(),
            BlockFinality::NonFinal(block) => block.ethereum_block.block.cheap_clone(),
        }
    }
}

impl<'a> From<&'a BlockFinality> for BlockPtr {
    fn from(block: &'a BlockFinality) -> BlockPtr {
        match block {
            BlockFinality::Final(b) => BlockPtr::from(&**b),
            BlockFinality::NonFinal(b) => BlockPtr::from(&b.ethereum_block),
        }
    }
}

impl Block for BlockFinality {
    fn ptr(&self) -> BlockPtr {
        match self {
            BlockFinality::Final(block) => block.block_ptr(),
            BlockFinality::NonFinal(block) => block.ethereum_block.block.block_ptr(),
        }
    }

    fn parent_ptr(&self) -> Option<BlockPtr> {
        match self {
            BlockFinality::Final(block) => block.parent_ptr(),
            BlockFinality::NonFinal(block) => block.ethereum_block.block.parent_ptr(),
        }
    }
}
*/
