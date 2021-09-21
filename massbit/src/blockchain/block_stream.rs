use anyhow::Error;
use async_trait::async_trait;
use futures03::Stream;

use super::{Block, BlockPtr, Blockchain};
use crate::components::store::BlockNumber;

pub trait BlockStream<C: Blockchain>:
    Stream<Item = Result<BlockStreamEvent<C>, Error>> + Unpin
{
}

pub struct BlockWithTriggers<C: Blockchain> {
    pub block: C::Block,
    pub trigger_data: Vec<C::TriggerData>,
}

impl<C: Blockchain> BlockWithTriggers<C> {
    pub fn new(block: C::Block, mut trigger_data: Vec<C::TriggerData>) -> Self {
        trigger_data.sort();
        Self {
            block,
            trigger_data,
        }
    }

    pub fn trigger_count(&self) -> usize {
        self.trigger_data.len()
    }

    pub fn ptr(&self) -> BlockPtr {
        self.block.ptr()
    }
}

#[async_trait]
pub trait TriggersAdapter<C: Blockchain>: Send + Sync {}

pub enum BlockStreamEvent<C: Blockchain> {
    ProcessBlock(BlockWithTriggers<C>),
}
