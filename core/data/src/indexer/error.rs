use crate::indexer::DeploymentHash;
use crate::store::chain::BlockPtr;
use massbit_common::prelude::stable_hash::{SequenceNumber, StableHash, StableHasher};
use std::fmt;
use std::fmt::Display;

#[derive(Debug)]
pub struct IndexerError {
    pub subgraph_id: DeploymentHash,
    pub message: String,
    pub block_ptr: Option<BlockPtr>,
    pub handler: Option<String>,

    // `true` if we are certain the error is deterministic. If in doubt, this is `false`.
    pub deterministic: bool,
}

impl Display for IndexerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.message)?;
        if let Some(handler) = &self.handler {
            write!(f, " in handler `{}`", handler)?;
        }
        if let Some(block_ptr) = &self.block_ptr {
            write!(f, " at block {}", block_ptr)?;
        }
        Ok(())
    }
}

impl StableHash for IndexerError {
    fn stable_hash<H: StableHasher>(&self, mut sequence_number: H::Seq, state: &mut H) {
        let IndexerError {
            subgraph_id,
            message,
            block_ptr,
            handler,
            deterministic,
        } = self;
        subgraph_id.stable_hash(sequence_number.next_child(), state);
        message.stable_hash(sequence_number.next_child(), state);
        block_ptr.stable_hash(sequence_number.next_child(), state);
        handler.stable_hash(sequence_number.next_child(), state);
        deterministic.stable_hash(sequence_number.next_child(), state);
    }
}
