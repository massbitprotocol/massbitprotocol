use web3::types::H256;

use crate::prelude::BlockPtr;

/// Light wrapper around `EthereumBlockPointer` that is compatible with GraphQL values.
#[derive(Debug)]
pub struct EthereumBlock(BlockPtr);

impl EthereumBlock {
    pub fn new(hash: H256, number: u64) -> Self {
        EthereumBlock(BlockPtr::from((hash, number)))
    }

    pub fn to_ptr(self) -> BlockPtr {
        self.0
    }

    pub fn number(&self) -> i32 {
        self.0.number
    }
}
