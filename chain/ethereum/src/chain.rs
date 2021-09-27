use massbit::prelude::*;
use std::sync::Arc;

use massbit::blockchain::{
    block_stream::BlockWithTriggers, Block, BlockStream, Blockchain, BlockchainKind,
    PollingBlockStream, TriggersAdapter as TriggersAdapterTrait,
};

use crate::data_source::{
    DataSource, DataSourceTemplate, UnresolvedDataSource, UnresolvedDataSourceTemplate,
};
use crate::ethereum_adapter::blocks_with_triggers;
use crate::network::{EthereumNetworkAdapter, EthereumNetworkAdapters};
use crate::types::{LightEthereumBlock, LightEthereumBlockExt};
use crate::TriggerFilter;
use crate::{EthereumAdapter, RuntimeAdapter};
use anyhow::Context;
use massbit::components::store::DeploymentLocator;

pub struct Chain {
    pub eth_adapters: Arc<EthereumNetworkAdapters>,
}

impl std::fmt::Debug for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "chain: ethereum")
    }
}

#[async_trait]
impl Blockchain for Chain {
    const KIND: BlockchainKind = BlockchainKind::Ethereum;

    type Block = BlockFinality;

    type DataSource = DataSource;

    type UnresolvedDataSource = UnresolvedDataSource;

    type DataSourceTemplate = DataSourceTemplate;

    type UnresolvedDataSourceTemplate = UnresolvedDataSourceTemplate;

    type TriggersAdapter = TriggersAdapter;

    type TriggerData = crate::trigger::EthereumTrigger;

    type MappingTrigger = crate::trigger::MappingTrigger;

    type TriggerFilter = crate::adapter::TriggerFilter;

    type RuntimeAdapter = RuntimeAdapter;

    fn triggers_adapter(&self) -> Result<Arc<Self::TriggersAdapter>, Error> {
        let eth_adapter = self
            .eth_adapters
            .cheapest()
            .with_context(|| "no adapter for chain")?
            .clone();
        let adapter = TriggersAdapter { eth_adapter };
        Ok(Arc::new(adapter))
    }

    fn runtime_adapter(&self) -> Arc<Self::RuntimeAdapter> {
        Arc::new(RuntimeAdapter {
            eth_adapters: self.eth_adapters.cheap_clone(),
        })
    }

    async fn new_block_stream(
        &self,
        deployment: DeploymentLocator,
        start_block: BlockNumber,
        filter: Arc<Self::TriggerFilter>,
    ) -> Result<Box<dyn BlockStream<Self>>, Error> {
        let triggers_adapter = self.triggers_adapter()?;
        Ok(Box::new(PollingBlockStream::new(
            triggers_adapter,
            filter,
            start_block,
        )))
    }
}

/// This is used in `EthereumAdapter::triggers_in_block`, called when re-processing a block for
/// newly created data sources. This allows the re-processing to be reorg safe without having to
/// always fetch the full block data.
#[derive(Clone, Debug)]
pub enum BlockFinality {
    /// If a block is final, we only need the header and the triggers.
    Final(Arc<LightEthereumBlock>),
}

impl BlockFinality {
    pub(crate) fn light_block(&self) -> Arc<LightEthereumBlock> {
        match self {
            BlockFinality::Final(block) => block.clone(),
        }
    }
}

impl<'a> From<&'a BlockFinality> for BlockPtr {
    fn from(block: &'a BlockFinality) -> BlockPtr {
        match block {
            BlockFinality::Final(b) => BlockPtr::from(&**b),
        }
    }
}

impl Block for BlockFinality {
    fn ptr(&self) -> BlockPtr {
        match self {
            BlockFinality::Final(block) => block.block_ptr(),
        }
    }

    fn parent_ptr(&self) -> Option<BlockPtr> {
        match self {
            BlockFinality::Final(block) => block.parent_ptr(),
        }
    }
}

pub struct TriggersAdapter {
    eth_adapter: Arc<EthereumAdapter>,
}

#[async_trait]
impl TriggersAdapterTrait<Chain> for TriggersAdapter {
    async fn scan_triggers(
        &self,
        from: BlockNumber,
        to: BlockNumber,
        filter: &TriggerFilter,
    ) -> Result<Vec<BlockWithTriggers<Chain>>, Error> {
        blocks_with_triggers(self.eth_adapter.clone(), from, to, filter).await
    }

    async fn triggers_in_block(
        &self,
        block: BlockFinality,
        filter: &TriggerFilter,
    ) -> Result<BlockWithTriggers<Chain>, Error> {
        match &block {
            BlockFinality::Final(_) => {
                let block_number = block.number() as BlockNumber;
                let blocks = blocks_with_triggers(
                    self.eth_adapter.clone(),
                    block_number,
                    block_number,
                    filter,
                )
                .await?;
                assert!(blocks.len() == 1);
                Ok(blocks.into_iter().next().unwrap())
            }
        }
    }
}
