use crate::types::LightEthereumBlock;
use ethabi::{Function, ParamType, Token};
use mockall::automock;
use mockall::predicate::*;
use std::collections::HashSet;
use web3::types::{Address, H256};

use massbit::blockchain::types::BlockPtr;
use massbit::components::store::EthereumCallCache;
use massbit::prelude::*;

#[derive(Clone, Debug)]
pub struct EthereumContractCall {
    pub address: Address,
    pub block_ptr: BlockPtr,
    pub function: Function,
    pub args: Vec<Token>,
}

#[derive(Error, Debug)]
pub enum EthereumContractCallError {
    #[error("ABI error: {0}")]
    ABIError(ABIError),
    /// `Token` is not of expected `ParamType`
    #[error("type mismatch, token {0:?} is not of kind {0:?}")]
    TypeError(Token, ParamType),
    #[error("error encoding input call data: {0}")]
    EncodingError(ethabi::Error),
    #[error("call error: {0}")]
    Web3Error(web3::Error),
    #[error("call reverted: {0}")]
    Revert(String),
    #[error("ethereum node took too long to perform call")]
    Timeout,
}

impl From<ABIError> for EthereumContractCallError {
    fn from(e: ABIError) -> Self {
        EthereumContractCallError::ABIError(e)
    }
}

/// Common trait for components that watch and manage access to Ethereum.
///
/// Implementations may be implemented against an in-process Ethereum node
/// or a remote node over RPC.
#[automock]
#[async_trait]
pub trait EthereumAdapter: Send + Sync + 'static {
    /// Get the latest block, including full transactions.
    fn latest_block(
        &self,
    ) -> Box<dyn Future<Item = LightEthereumBlock, Error = Error> + Send + Unpin>;

    /// Load Ethereum blocks in bulk, returning results as they come back as a Stream.
    fn load_blocks(
        &self,
        block_hashes: HashSet<H256>,
    ) -> Box<dyn Stream<Item = LightEthereumBlock, Error = Error> + Send>;

    /// Call the function of a smart contract.
    fn contract_call(
        &self,
        call: EthereumContractCall,
        cache: Arc<dyn EthereumCallCache>,
    ) -> Box<dyn Future<Item = Vec<Token>, Error = EthereumContractCallError> + Send>;
}
