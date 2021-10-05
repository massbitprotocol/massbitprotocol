use ethabi::{Error as ABIError, Function, ParamType, Token};
use std::cmp;
use std::collections::{HashMap, HashSet};
use std::fmt;
use thiserror::Error;
use tiny_keccak::keccak256;
use web3::types::{Address, H256};

use massbit::blockchain as bc;
use massbit::prelude::*;

use crate::chain::Chain;
use crate::data_source::{BlockHandlerFilter, DataSource};

pub type EventSignature = H256;
pub type FunctionSelector = [u8; 4];

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
enum LogFilterNode {
    Contract(Address),
    Event(EventSignature),
}

/// Corresponds to an `eth_getLogs` call.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EthGetLogsFilter {
    pub contracts: Vec<Address>,
    pub event_signatures: Vec<EventSignature>,
}

impl fmt::Display for EthGetLogsFilter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.contracts.len() == 1 {
            write!(
                f,
                "contract {:?}, {} events",
                self.contracts[0],
                self.event_signatures.len()
            )
        } else if self.event_signatures.len() == 1 {
            write!(
                f,
                "event {:?}, {} contracts",
                self.event_signatures[0],
                self.contracts.len()
            )
        } else {
            write!(f, "unreachable")
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TriggerFilter {
    pub log: EthereumLogFilter,
    pub call: EthereumCallFilter,
    pub block: EthereumBlockFilter,
}

impl bc::TriggerFilter<Chain> for TriggerFilter {
    fn from_data_sources<'a>(data_sources: impl Iterator<Item = &'a DataSource> + Clone) -> Self {
        let mut this = Self::default();
        this.extend(data_sources);
        this
    }

    fn extend<'a>(&mut self, data_sources: impl Iterator<Item = &'a DataSource> + Clone) {
        self.log
            .extend(EthereumLogFilter::from_data_sources(data_sources.clone()));
        self.call
            .extend(EthereumCallFilter::from_data_sources(data_sources.clone()));
        self.block
            .extend(EthereumBlockFilter::from_data_sources(data_sources));
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EthereumLogFilter {
    pub eth_logs_filters: Vec<EthGetLogsFilter>,
}

impl EthereumLogFilter {
    pub fn from_data_sources<'a>(iter: impl IntoIterator<Item = &'a DataSource>) -> Self {
        let mut this = EthereumLogFilter::default();
        for ds in iter {
            for event_sig in ds.mapping.event_handlers.iter().map(|e| e.topic0()) {
                match ds.source.address {
                    Some(contract) => {
                        this.eth_logs_filters.push(EthGetLogsFilter {
                            contracts: vec![contract],
                            event_signatures: vec![event_sig],
                        });
                    }
                    None => {
                        this.eth_logs_filters.push(EthGetLogsFilter {
                            contracts: vec![],
                            event_signatures: vec![event_sig],
                        });
                    }
                }
            }
        }
        this
    }

    /// Extends this log filter with another one.
    pub fn extend(&mut self, other: EthereumLogFilter) {
        for filter in other.eth_logs_filters {
            self.eth_logs_filters.push(filter)
        }
    }

    /// An empty filter is one that never matches.
    pub fn is_empty(&self) -> bool {
        self.eth_logs_filters.is_empty()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EthereumCallFilter {
    // Each call filter has a map of filters keyed by address, each containing a tuple with
    // start_block and the set of function signatures
    pub contract_addresses_function_signatures:
        HashMap<Address, (BlockNumber, HashSet<FunctionSelector>)>,
}

impl EthereumCallFilter {
    pub fn matches(&self, call: &EthereumCall) -> bool {
        // Ensure the call is to a contract the filter expressed an interest in
        if !self
            .contract_addresses_function_signatures
            .contains_key(&call.to)
        {
            return false;
        }
        // If the call is to a contract with no specified functions, keep the call
        if self
            .contract_addresses_function_signatures
            .get(&call.to)
            .unwrap()
            .1
            .is_empty()
        {
            // Allow the ability to match on calls to a contract generally
            // If you want to match on a generic call to contract this limits you
            // from matching with a specific call to a contract
            return true;
        }
        // Ensure the call is to run a function the filter expressed an interest in
        self.contract_addresses_function_signatures
            .get(&call.to)
            .unwrap()
            .1
            .contains(&call.input.0[..4])
    }

    pub fn from_data_sources<'a>(iter: impl IntoIterator<Item = &'a DataSource>) -> Self {
        iter.into_iter()
            .filter_map(|data_source| data_source.source.address.map(|addr| (addr, data_source)))
            .map(|(contract_addr, data_source)| {
                let start_block = data_source.source.start_block;
                data_source
                    .mapping
                    .call_handlers
                    .iter()
                    .map(move |call_handler| {
                        let sig = keccak256(call_handler.function.as_bytes());
                        (start_block, contract_addr, [sig[0], sig[1], sig[2], sig[3]])
                    })
            })
            .flatten()
            .collect()
    }

    /// Extends this call filter with another one.
    pub fn extend(&mut self, other: EthereumCallFilter) {
        // Extend existing address / function signature key pairs
        // Add new address / function signature key pairs from the provided EthereumCallFilter
        for (address, (proposed_start_block, new_sigs)) in
            other.contract_addresses_function_signatures.into_iter()
        {
            match self
                .contract_addresses_function_signatures
                .get_mut(&address)
            {
                Some((existing_start_block, existing_sigs)) => {
                    *existing_start_block =
                        cmp::min(proposed_start_block, existing_start_block.clone());
                    existing_sigs.extend(new_sigs);
                }
                None => {
                    self.contract_addresses_function_signatures
                        .insert(address, (proposed_start_block, new_sigs));
                }
            }
        }
    }

    /// An empty filter is one that never matches.
    pub fn is_empty(&self) -> bool {
        // Destructure to make sure we're checking all fields.
        let EthereumCallFilter {
            contract_addresses_function_signatures,
        } = self;
        contract_addresses_function_signatures.is_empty()
    }
}

impl FromIterator<(BlockNumber, Address, FunctionSelector)> for EthereumCallFilter {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (BlockNumber, Address, FunctionSelector)>,
    {
        let mut lookup: HashMap<Address, (BlockNumber, HashSet<FunctionSelector>)> = HashMap::new();
        iter.into_iter()
            .for_each(|(start_block, address, function_signature)| {
                if !lookup.contains_key(&address) {
                    lookup.insert(address, (start_block, HashSet::default()));
                }
                lookup.get_mut(&address).map(|set| {
                    if set.0 > start_block {
                        set.0 = start_block
                    }
                    set.1.insert(function_signature);
                    set
                });
            });
        EthereumCallFilter {
            contract_addresses_function_signatures: lookup,
        }
    }
}

impl From<EthereumBlockFilter> for EthereumCallFilter {
    fn from(ethereum_block_filter: EthereumBlockFilter) -> Self {
        Self {
            contract_addresses_function_signatures: ethereum_block_filter
                .contract_addresses
                .into_iter()
                .map(|(start_block_opt, address)| (address, (start_block_opt, HashSet::default())))
                .collect::<HashMap<Address, (BlockNumber, HashSet<FunctionSelector>)>>(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EthereumBlockFilter {
    pub contract_addresses: HashSet<(BlockNumber, Address)>,
    pub trigger_every_block: bool,
}

impl EthereumBlockFilter {
    pub fn from_data_sources<'a>(iter: impl IntoIterator<Item = &'a DataSource>) -> Self {
        iter.into_iter()
            .filter(|data_source| data_source.source.address.is_some())
            .fold(Self::default(), |mut filter_opt, data_source| {
                let has_block_handler_with_call_filter = data_source
                    .mapping
                    .block_handlers
                    .clone()
                    .into_iter()
                    .any(|block_handler| match block_handler.filter {
                        Some(ref filter) if *filter == BlockHandlerFilter::Call => return true,
                        _ => return false,
                    });

                let has_block_handler_without_filter = data_source
                    .mapping
                    .block_handlers
                    .clone()
                    .into_iter()
                    .any(|block_handler| block_handler.filter.is_none());

                filter_opt.extend(Self {
                    trigger_every_block: has_block_handler_without_filter,
                    contract_addresses: if has_block_handler_with_call_filter {
                        vec![(
                            data_source.source.start_block,
                            data_source.source.address.unwrap().to_owned(),
                        )]
                        .into_iter()
                        .collect()
                    } else {
                        HashSet::default()
                    },
                });
                filter_opt
            })
    }

    pub fn extend(&mut self, other: EthereumBlockFilter) {
        self.trigger_every_block = self.trigger_every_block || other.trigger_every_block;
        self.contract_addresses = self.contract_addresses.iter().cloned().fold(
            HashSet::new(),
            |mut addresses, (start_block, address)| {
                match other
                    .contract_addresses
                    .iter()
                    .cloned()
                    .find(|(_, other_address)| &address == other_address)
                {
                    Some((other_start_block, address)) => {
                        addresses.insert((cmp::min(other_start_block, start_block), address));
                    }
                    None => {
                        addresses.insert((start_block, address));
                    }
                }
                addresses
            },
        );
    }
}

/// Common trait for components that watch and manage access to Ethereum.
///
/// Implementations may be implemented against an in-process Ethereum node
/// or a remote node over RPC.
#[async_trait]
pub trait EthereumAdapter: Send + Sync + 'static {
    fn provider(&self) -> &str;

    async fn net_identifiers(&self) -> Result<EthereumNetworkIdentifier, Error>;

    fn block_hash_by_block_number(
        &self,
        logger: &Logger,
        block_number: BlockNumber,
    ) -> Box<dyn Future<Item = Option<H256>, Error = Error> + Send>;

    fn load_blocks(
        &self,
        logger: Logger,
        block_hashes: HashSet<H256>,
    ) -> Box<dyn Stream<Item = LightEthereumBlock, Error = Error> + Send>;
}
