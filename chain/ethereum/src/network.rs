use std::collections::HashMap;
use std::sync::Arc;

use crate::adapter::EthereumAdapter as _;
use crate::EthereumAdapter;

#[derive(Clone)]
pub struct EthereumNetworkAdapter {
    pub adapter: Arc<EthereumAdapter>,
}

#[derive(Clone)]
pub struct EthereumNetworkAdapters {
    pub adapters: Vec<EthereumNetworkAdapter>,
}

impl EthereumNetworkAdapters {
    pub fn remove(&mut self, provider: &str) {
        self.adapters
            .retain(|adapter| adapter.adapter.provider() != provider);
    }
}

#[derive(Clone)]
pub struct EthereumNetworks {
    pub networks: HashMap<String, EthereumNetworkAdapters>,
}

impl EthereumNetworks {
    pub fn new() -> EthereumNetworks {
        EthereumNetworks {
            networks: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: String, adapter: Arc<EthereumAdapter>) {
        let network_adapters = self
            .networks
            .entry(name)
            .or_insert(EthereumNetworkAdapters { adapters: vec![] });
        network_adapters.adapters.push(EthereumNetworkAdapter {
            adapter: adapter.clone(),
        });
    }

    pub fn remove(&mut self, name: &str, provider: &str) {
        if let Some(adapters) = self.networks.get_mut(name) {
            adapters.remove(provider);
        }
    }

    pub fn extend(&mut self, other_networks: EthereumNetworks) {
        self.networks.extend(other_networks.networks);
    }

    pub fn flatten(&self) -> Vec<(String, Arc<EthereumAdapter>)> {
        self.networks
            .iter()
            .flat_map(|(network_name, network_adapters)| {
                network_adapters
                    .adapters
                    .iter()
                    .map(move |network_adapter| {
                        (network_name.clone(), network_adapter.adapter.clone())
                    })
            })
            .collect()
    }
}
