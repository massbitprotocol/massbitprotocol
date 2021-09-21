use crate::blockchain::block_stream::BlockWithTriggers;
use crate::blockchain::Blockchain;
use crate::prelude::*;
use futures03::{stream::Stream, Future, FutureExt};
use std::collections::VecDeque;

#[cfg(debug_assertions)]
use fail::fail_point;
enum BlockStreamState<C>
where
    C: Blockchain,
{
    /// Starting or restarting reconciliation.
    ///
    /// Valid next states: Reconciliation
    BeginReconciliation,

    /// The BlockStream is reconciling the indexer store state with the chain store state.
    ///
    /// Valid next states: YieldingBlocks, Idle
    Reconciliation(Pin<Box<dyn Future<Output = Result<NextBlocks<C>, Error>> + Send>>),

    /// The BlockStream is emitting blocks that must be processed in order to bring the indexer
    /// store up to date with the chain store.
    ///
    /// Valid next states: BeginReconciliation
    YieldingBlocks(Box<VecDeque<BlockWithTriggers<C>>>),

    /// The BlockStream has reconciled the indexer store and chain store states.
    /// No more work is needed until a chain head update.
    ///
    /// Valid next states: BeginReconciliation
    Idle,
}

/// A single next step to take in reconciling the state of the indexer store with the state of the
/// chain store.
enum ReconciliationStep<C>
where
    C: Blockchain,
{
    /// Move forwards, processing one or more blocks. Second element is the block range size.
    ProcessDescendantBlocks(Vec<BlockWithTriggers<C>>, BlockNumber),

    /// This step is a no-op, but we need to check again for a next step.
    Retry,

    /// indexer pointer now matches chain head pointer.
    /// Reconciliation is complete.
    Done,
}

struct BlockStreamContext<C>
where
    C: Blockchain,
{
    adapter: Arc<C::TriggersAdapter>,
    filter: Arc<C::TriggerFilter>,
    start_blocks: Vec<BlockNumber>,
    previous_triggers_per_block: f64,
    // Not a BlockNumber, but the difference between two block numbers
    previous_block_range_size: BlockNumber,
    // Not a BlockNumber, but the difference between two block numbers
    max_block_range_size: BlockNumber,
    target_triggers_per_block_range: u64,
}

impl<C: Blockchain> Clone for BlockStreamContext<C> {
    fn clone(&self) -> Self {
        Self {
            adapter: self.adapter.clone(),
            filter: self.filter.clone(),
            start_blocks: self.start_blocks.clone(),
            previous_triggers_per_block: self.previous_triggers_per_block,
            previous_block_range_size: self.previous_block_range_size,
            max_block_range_size: self.max_block_range_size,
            target_triggers_per_block_range: self.target_triggers_per_block_range,
        }
    }
}

pub struct PollingBlockStream<C: Blockchain> {
    state: BlockStreamState<C>,
    ctx: BlockStreamContext<C>,
}

// This is the same as `ReconciliationStep` but without retries.
enum NextBlocks<C>
where
    C: Blockchain,
{
    /// Blocks and range size
    Blocks(VecDeque<BlockWithTriggers<C>>, BlockNumber),

    Done,
}

impl<C> PollingBlockStream<C>
where
    C: Blockchain,
{
    pub fn new(
        adapter: Arc<C::TriggersAdapter>,
        filter: Arc<C::TriggerFilter>,
        start_blocks: Vec<BlockNumber>,
        max_block_range_size: BlockNumber,
        target_triggers_per_block_range: u64,
    ) -> Self {
        PollingBlockStream {
            state: BlockStreamState::BeginReconciliation,
            ctx: BlockStreamContext {
                adapter,
                filter,
                start_blocks,

                // A high number here forces a slow start, with a range of 1.
                previous_triggers_per_block: 1_000_000.0,
                previous_block_range_size: 1,
                max_block_range_size,
                target_triggers_per_block_range,
            },
        }
    }
}

// impl<C> BlockStreamContext<C>
// where
//     C: Blockchain,
// {
//     async fn next_blocks(&self) -> Result<NextBlocks<C>, Error> {
//         let ctx = self.clone();
//
//         loop {
//             match ctx.get_next_step().await? {
//                 ReconciliationStep::ProcessDescendantBlocks(next_blocks, range_size) => {
//                     return Ok(NextBlocks::Blocks(
//                         next_blocks.into_iter().collect(),
//                         range_size,
//                     ));
//                 }
//                 ReconciliationStep::Retry => {
//                     continue;
//                 }
//                 ReconciliationStep::Done => {
//                     return Ok(NextBlocks::Done);
//                 }
//             }
//         }
//     }
//
//     /// Determine the next reconciliation step. Does not modify Store or ChainStore.
//     async fn get_next_step(&self) -> Result<ReconciliationStep<C>, Error> {
//         let ctx = self.clone();
//         let start_blocks = self.start_blocks.clone();
//         let max_block_range_size = self.max_block_range_size;
//     }
// }
