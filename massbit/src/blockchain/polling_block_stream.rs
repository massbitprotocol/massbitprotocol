use futures03::{stream::Stream, Future, FutureExt};
use std::collections::VecDeque;
use std::task::{Context, Poll};

use crate::blockchain::block_stream::{BlockStreamEvent, BlockWithTriggers, TriggersAdapter};
use crate::blockchain::{BlockStream, Blockchain};
use crate::components::store::{ChainStore, BLOCK_NUMBER_MAX};
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
    adapter: Arc<C::TriggersAdapter>,
    filter: Arc<C::TriggerFilter>,
    stream_block_number: BlockNumber,
}

impl<C: Blockchain> Clone for BlockStreamContext<C> {
    fn clone(&self) -> Self {
        Self {
            adapter: self.adapter.clone(),
            filter: self.filter.clone(),
            stream_block_number: self.stream_block_number,
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
        adapter: Arc<C::TriggersAdapter>,
        filter: Arc<C::TriggerFilter>,
        start_block: BlockNumber,
    ) -> Self {
        PollingBlockStream {
            state: BlockStreamState::BeginReconciliation,
            ctx: BlockStreamContext {
                adapter,
                filter,
                stream_block_number: start_block,
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
        let from = self.stream_block_number;

        let blocks = self
            .adapter
            .scan_triggers(from, from + *STREAM_BLOCK_RANGE_SIZE, &self.filter)
            .await?;

        Ok(ReconciliationStep::ProcessDescendantBlocks(
            blocks,
            *STREAM_BLOCK_RANGE_SIZE,
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
                            if total_triggers > 0 {
                                info!("Processing {} triggers", total_triggers);
                            }

                            self.ctx.stream_block_number =
                                self.ctx.stream_block_number + block_range_size + 1;

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
