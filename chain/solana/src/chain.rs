use super::data_source::DataSource;
use crate::adapter::SolanaAdapter;
use crate::data_source::{DataSourceTemplate, UnresolvedDataSource, UnresolvedDataSourceTemplate};
use crate::trigger::TriggerFilter;
use massbit::blockchain::block_stream::BlockWithTriggers;
use massbit::blockchain::{
    Block, BlockPtr, BlockStream, Blockchain, BlockchainKind,
    TriggersAdapter as TriggersAdapterTrait,
};
use massbit::components::store::BlockNumber;
use massbit::prelude::{async_trait, Arc, Deserialize, Error, Logger, LoggerFactory, Serialize};
use solana_transaction_status::EncodedConfirmedBlock;

pub struct Chain {
    logger_factory: LoggerFactory,
    name: String,
}

impl std::fmt::Debug for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "chain: solana")
    }
}

impl Chain {}
#[async_trait]
impl Blockchain for Chain {
    const KIND: BlockchainKind = BlockchainKind::Ethereum;
    type Block = SolanaBlock;
    type DataSource = DataSource;
    type UnresolvedDataSource = UnresolvedDataSource;
    type DataSourceTemplate = DataSourceTemplate;
    type UnresolvedDataSourceTemplate = UnresolvedDataSourceTemplate;
    type TriggersAdapter = TriggersAdapter;
    type TriggerData = crate::trigger::SolanaTriggerData;
    type MappingTrigger = crate::trigger::SolanaMappingTrigger;
    type TriggerFilter = crate::trigger::TriggerFilter;
    type RuntimeAdapter = crate::adapter::RuntimeAdapter;

    fn triggers_adapter(&self) -> Result<Arc<Self::TriggersAdapter>, Error> {
        todo!()
    }

    fn runtime_adapter(&self) -> Arc<Self::RuntimeAdapter> {
        todo!()
    }

    async fn new_block_stream(
        &self,
        start_block: BlockNumber,
        filter: Arc<Self::TriggerFilter>,
    ) -> Result<Box<dyn BlockStream<Self>>, Error> {
        todo!()
    }

    async fn block_pointer_from_number(
        &self,
        logger: &Logger,
        number: BlockNumber,
    ) -> Result<BlockPtr, Error> {
        todo!()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SolanaBlock {
    Encoded(Arc<EncodedConfirmedBlock>),
}

impl SolanaBlock {}

impl Block for SolanaBlock {
    fn ptr(&self) -> BlockPtr {
        todo!()
    }

    fn parent_ptr(&self) -> Option<BlockPtr> {
        todo!()
    }
}

pub struct TriggersAdapter {
    logger: Logger,
    chain_adapter: Arc<SolanaAdapter>,
}

#[async_trait]
impl TriggersAdapterTrait<Chain> for TriggersAdapter {
    async fn scan_triggers(
        &self,
        from: BlockNumber,
        to: BlockNumber,
        filter: &TriggerFilter,
    ) -> Result<Vec<BlockWithTriggers<Chain>>, Error> {
        todo!()
    }

    async fn triggers_in_block(
        &self,
        logger: &Logger,
        block: SolanaBlock,
        filter: &TriggerFilter,
    ) -> Result<BlockWithTriggers<Chain>, Error> {
        todo!()
    }
}
// #[async_trait]
// impl TriggersAdapterTrait<Chain> for TriggersAdapter {
//     async fn scan_triggers(
//         &self,
//         from: BlockNumber,
//         to: BlockNumber,
//         filter: &TriggerFilter,
//     ) -> Result<Vec<BlockWithTriggers<Chain>>, Error> {
//         blocks_with_triggers(
//             self.logger.clone(),
//             self.eth_adapter.clone(),
//             from,
//             to,
//             filter,
//         )
//         .await
//     }
//
//     async fn triggers_in_block(
//         &self,
//         logger: &Logger,
//         block: BlockFinality,
//         filter: &TriggerFilter,
//     ) -> Result<BlockWithTriggers<Chain>, Error> {
//         match &block {
//             BlockFinality::Final(_) => {
//                 let block_number = block.number() as BlockNumber;
//                 let blocks = blocks_with_triggers(
//                     logger.clone(),
//                     self.eth_adapter.clone(),
//                     block_number,
//                     block_number,
//                     filter,
//                 )
//                 .await?;
//                 assert!(blocks.len() == 1);
//                 Ok(blocks.into_iter().next().unwrap())
//             }
//         }
//     }
// }
