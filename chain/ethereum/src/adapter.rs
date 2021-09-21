use ethabi::{Function, ParamType, Token};
use std::cmp;
use std::collections::{HashMap, HashSet};
use std::fmt;
use tiny_keccak::keccak256;
use web3::types::{Address, Log, H256};

use massbit::prelude::*;
use massbit::{
    blockchain as bc,
    petgraph::{self, graphmap::GraphMap},
};

use crate::chain::Chain;
use crate::data_source::{BlockHandlerFilter, DataSource};
use crate::types::LightEthereumBlock;

pub type EventSignature = H256;
pub type FunctionSelector = [u8; 4];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
enum LogFilterNode {
    Contract(Address),
    Event(EventSignature),
}

/// Corresponds to an `eth_getLogs` call.
#[derive(Clone)]
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

#[derive(Clone, Debug, Default)]
pub struct TriggerFilter {
    pub(crate) log: EthereumLogFilter,
    pub(crate) call: EthereumCallFilter,
    pub(crate) block: EthereumBlockFilter,
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

#[derive(Clone, Debug, Default)]
pub(crate) struct EthereumLogFilter {
    /// Log filters can be represented as a bipartite graph between contracts and events. An edge
    /// exists between a contract and an event if a data source for the contract has a trigger for
    /// the event.
    contracts_and_events_graph: GraphMap<LogFilterNode, (), petgraph::Undirected>,

    // Event sigs with no associated address, matching on all addresses.
    wildcard_events: HashSet<EventSignature>,
}

impl EthereumLogFilter {
    pub fn from_data_sources<'a>(iter: impl IntoIterator<Item = &'a DataSource>) -> Self {
        let mut this = EthereumLogFilter::default();
        for ds in iter {
            for event_sig in ds.mapping.event_handlers.iter().map(|e| e.topic0()) {
                match ds.source.address {
                    Some(contract) => {
                        this.contracts_and_events_graph.add_edge(
                            LogFilterNode::Contract(contract),
                            LogFilterNode::Event(event_sig),
                            (),
                        );
                    }
                    None => {
                        this.wildcard_events.insert(event_sig);
                    }
                }
            }
        }
        this
    }

    /// Extends this log filter with another one.
    pub fn extend(&mut self, other: EthereumLogFilter) {
        // Destructure to make sure we're checking all fields.
        let EthereumLogFilter {
            contracts_and_events_graph,
            wildcard_events,
        } = other;
        for (s, t, ()) in contracts_and_events_graph.all_edges() {
            self.contracts_and_events_graph.add_edge(s, t, ());
        }
        self.wildcard_events.extend(wildcard_events);
    }

    /// An empty filter is one that never matches.
    pub fn is_empty(&self) -> bool {
        // Destructure to make sure we're checking all fields.
        let EthereumLogFilter {
            contracts_and_events_graph,
            wildcard_events,
        } = self;
        contracts_and_events_graph.edge_count() == 0 && wildcard_events.is_empty()
    }

    /// Filters for `eth_getLogs` calls. The filters will not return false positives. This attempts
    /// to balance between having granular filters but too many calls and having few calls but too
    /// broad filters causing the Ethereum endpoint to timeout.
    pub fn eth_get_logs_filters(self) -> impl Iterator<Item = EthGetLogsFilter> {
        let mut filters = Vec::new();

        // First add the wildcard event filters.
        for wildcard_event in self.wildcard_events {
            filters.push(EthGetLogsFilter {
                contracts: vec![],
                event_signatures: vec![wildcard_event],
            })
        }

        // The current algorithm is to repeatedly find the maximum cardinality vertex and turn all
        // of its edges into a filter. This is nice because it is neutral between filtering by
        // contract or by events, if there are many events that appear on only one data source
        // we'll filter by many events on a single contract, but if there is an event that appears
        // on a lot of data sources we'll filter by many contracts with a single event.
        //
        // From a theoretical standpoint we're finding a vertex cover, and this is not the optimal
        // algorithm to find a minimum vertex cover, but should be fine as an approximation.
        //
        // One optimization we're not doing is to merge nodes that have the same neighbors into a
        // single node. For example if a subgraph has two data sources, each with the same two
        // events, we could cover that with a single filter and no false positives. However that
        // might cause the filter to become too broad, so at the moment it seems excessive.
        let mut g = self.contracts_and_events_graph;
        while g.edge_count() > 0 {
            // If there are edges, there are vertexes.
            let max_vertex = g.nodes().max_by_key(|&n| g.neighbors(n).count()).unwrap();
            let mut filter = match max_vertex {
                LogFilterNode::Contract(address) => EthGetLogsFilter {
                    contracts: vec![address],
                    event_signatures: vec![],
                },
                LogFilterNode::Event(event_sig) => EthGetLogsFilter {
                    contracts: vec![],
                    event_signatures: vec![event_sig],
                },
            };
            for neighbor in g.neighbors(max_vertex) {
                match neighbor {
                    LogFilterNode::Contract(address) => filter.contracts.push(address),
                    LogFilterNode::Event(event_sig) => filter.event_signatures.push(event_sig),
                }
            }

            // Sanity checks:
            // - The filter is not a wildcard because all nodes have neighbors.
            // - The graph is bipartite.
            assert!(filter.contracts.len() > 0 && filter.event_signatures.len() > 0);
            assert!(filter.contracts.len() == 1 || filter.event_signatures.len() == 1);
            filters.push(filter);
            g.remove_node(max_vertex);
        }
        filters.into_iter()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct EthereumCallFilter {
    // Each call filter has a map of filters keyed by address, each containing a tuple with
    // start_block and the set of function signatures
    pub contract_addresses_function_signatures:
        HashMap<Address, (BlockNumber, HashSet<FunctionSelector>)>,
}

impl EthereumCallFilter {
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

#[derive(Clone, Debug, Default)]
pub(crate) struct EthereumBlockFilter {
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
    fn block_hash_by_block_number(
        &self,
        block_number: BlockNumber,
    ) -> Box<dyn Future<Item = Option<H256>, Error = Error> + Send>;

    fn load_blocks(
        &self,
        block_hashes: HashSet<H256>,
    ) -> Box<dyn Stream<Item = LightEthereumBlock, Error = Error> + Send>;
}
