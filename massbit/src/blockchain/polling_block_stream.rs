use futures03::{stream::Stream, Future, FutureExt};
use std::cmp;
use std::collections::VecDeque;
use std::task::{Context, Poll};

use crate::blockchain::block_stream::{BlockStreamEvent, BlockWithTriggers, TriggersAdapter};
use crate::blockchain::{BlockStream, Blockchain};
use crate::components::store::WritableStore;
use crate::prelude::*;

lazy_static! {
    pub static ref STREAM_BLOCK_RANGE_SIZE: i32 = std::env::var("STREAM_BLOCK_RANGE_SIZE")
        .ok()
        .map(|s| {
            s.parse::<i32>().unwrap_or_else(|_| {
                panic!("STREAM_BLOCK_RANGE_SIZE must be a number, but is `{}`", s)
            })
        })
        .unwrap_or(100);
}

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
    /// Valid next states: YieldingBlocks
    Reconciliation(Pin<Box<dyn Future<Output = Result<NextBlocks<C>, Error>> + Send>>),

    /// The BlockStream is emitting blocks that must be processed in order to bring the indexer
    /// store up to date with the chain store.
    ///
    /// Valid next states: BeginReconciliation
    YieldingBlocks(Box<VecDeque<BlockWithTriggers<C>>>),
}

/// A single next step to take in reconciling the state of the indexer store with the state of the
/// chain store.
enum ReconciliationStep<C>
where
    C: Blockchain,
{
    /// Move forwards, processing one or more blocks. Second element is the block range size.
    ProcessDescendantBlocks(Vec<BlockWithTriggers<C>>, BlockNumber),
}

struct BlockStreamContext<C>
where
    C: Blockchain,
{
    logger: Logger,
    indexer_store: Arc<dyn WritableStore>,
    adapter: Arc<C::TriggersAdapter>,
    filter: Arc<C::TriggerFilter>,
    // Current block pointer of stream
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
            indexer_store: self.indexer_store.cheap_clone(),
            logger: self.logger.clone(),
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
}

impl<C> PollingBlockStream<C>
where
    C: Blockchain,
{
    pub fn new(
        logger: Logger,
        indexer_store: Arc<dyn WritableStore>,
        adapter: Arc<C::TriggersAdapter>,
        filter: Arc<C::TriggerFilter>,
        start_blocks: Vec<BlockNumber>,
        max_block_range_size: BlockNumber,
        target_triggers_per_block_range: u64,
    ) -> Self {
        PollingBlockStream {
            state: BlockStreamState::BeginReconciliation,
            ctx: BlockStreamContext {
                logger,
                indexer_store,
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

impl<C> BlockStreamContext<C>
where
    C: Blockchain,
{
    async fn next_blocks(&self) -> Result<NextBlocks<C>, Error> {
        let ctx = self.clone();

        loop {
            match ctx.get_next_step().await? {
                ReconciliationStep::ProcessDescendantBlocks(next_blocks, range_size) => {
                    return Ok(NextBlocks::Blocks(
                        next_blocks.into_iter().collect(),
                        range_size,
                    ));
                }
            }
        }
    }

    /// Determine the next reconciliation step. Does not modify Store or ChainStore.
    async fn get_next_step(&self) -> Result<ReconciliationStep<C>, Error> {
        let ctx = self.clone();
        let start_blocks = self.start_blocks.clone();
        let max_block_range_size = self.max_block_range_size;

        let indexer_ptr = ctx.indexer_store.block_ptr()?;

        // Start with first block after subgraph ptr; if the ptr is None,
        // then we start with the genesis block
        let from = indexer_ptr.map_or(0, |ptr| ptr.number + 1);

        // Get the next subsequent data source start block to ensure the block
        // range is aligned with data source. This is not necessary for
        // correctness, but it avoids an ineffecient situation such as the range
        // being 0..100 and the start block for a data source being 99, then
        // `calls_in_block_range` would request unecessary traces for the blocks
        // 0 to 98 because the start block is within the range.
        let next_start_block: BlockNumber = start_blocks
            .into_iter()
            .filter(|block_num| block_num > &from)
            .min()
            .unwrap_or(BLOCK_NUMBER_MAX);

        let to_limit = next_start_block - 1;

        // Calculate the range size according to the target number of triggers,
        // respecting the global maximum and also not increasing too
        // drastically from the previous block range size.
        //
        // An example of the block range dynamics:
        // - Start with a block range of 1, target of 1000.
        // - Scan 1 block:
        //   0 triggers found, max_range_size = 10, range_size = 10
        // - Scan 10 blocks:
        //   2 triggers found, 0.2 per block, range_size = 1000 / 0.2 = 5000
        // - Scan 5000 blocks:
        //   10000 triggers found, 2 per block, range_size = 1000 / 2 = 500
        // - Scan 500 blocks:
        //   1000 triggers found, 2 per block, range_size = 1000 / 2 = 500
        let range_size_upper_limit = max_block_range_size.min(ctx.previous_block_range_size * 10);
        let range_size = if ctx.previous_triggers_per_block == 0.0 {
            range_size_upper_limit
        } else {
            (self.target_triggers_per_block_range as f64 / ctx.previous_triggers_per_block)
                .max(1.0)
                .min(range_size_upper_limit as f64) as BlockNumber
        };
        let to = cmp::min(from + range_size - 1, to_limit);

        info!(
            ctx.logger,
            "Scanning blocks [{}, {}]", from, to;
            "range_size" => range_size
        );

        let blocks = self.adapter.scan_triggers(from, to, &self.filter).await?;

        Ok(ReconciliationStep::ProcessDescendantBlocks(
            blocks, range_size,
        ))
    }
}

impl<C: Blockchain> BlockStream<C> for PollingBlockStream<C> {}

impl<C: Blockchain> Stream for PollingBlockStream<C> {
    type Item = Result<BlockStreamEvent<C>, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let result = loop {
            match &mut self.state {
                BlockStreamState::BeginReconciliation => {
                    // Start the reconciliation process by asking for blocks
                    let ctx = self.ctx.clone();
                    let fut = async move { ctx.next_blocks().await };
                    self.state = BlockStreamState::Reconciliation(fut.boxed());
                }

                // Waiting for the reconciliation to complete or yield blocks
                BlockStreamState::Reconciliation(next_blocks_future) => {
                    match next_blocks_future.poll_unpin(cx) {
                        Poll::Ready(Ok(NextBlocks::Blocks(next_blocks, block_range_size))) => {
                            let total_triggers =
                                next_blocks.iter().map(|b| b.trigger_count()).sum::<usize>();
                            self.ctx.previous_triggers_per_block =
                                total_triggers as f64 / block_range_size as f64;
                            self.ctx.previous_block_range_size = block_range_size;
                            if total_triggers > 0 {
                                debug!(self.ctx.logger, "Processing {} triggers", total_triggers);
                            }

                            // Switch to yielding state until next_blocks is depleted
                            self.state = BlockStreamState::YieldingBlocks(Box::new(next_blocks));

                            // Yield the first block in next_blocks
                            continue;
                        }
                        Poll::Pending => {
                            break Poll::Pending;
                        }
                        Poll::Ready(Err(e)) => {
                            break Poll::Ready(Some(Err(e)));
                        }
                    }
                }

                // Yielding blocks from reconciliation process
                BlockStreamState::YieldingBlocks(ref mut next_blocks) => {
                    match next_blocks.pop_front() {
                        // Yield one block
                        Some(next_block) => {
                            break Poll::Ready(Some(Ok(BlockStreamEvent::ProcessBlock(
                                next_block,
                            ))));
                        }

                        // Done yielding blocks
                        None => {
                            self.state = BlockStreamState::BeginReconciliation;
                        }
                    }
                }
            }
        };
        result
    }
}
