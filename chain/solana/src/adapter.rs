use super::types::ChainConfig;
use crate::chain::Chain;
use crate::data_source::DataSource;
use crate::types::{Pubkey, ResultFilterTransaction, SolanaFilter};
use crate::LIMIT_FILTER_RESULT;
use log::{debug, error, info, warn};
use massbit::blockchain as bc;
use massbit::blockchain::HostFn;
use massbit::prelude::*;
use solana_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient};
use solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature;
use solana_sdk::signature::Signature;
use std::str::FromStr;

#[derive(Clone)]
pub struct SolanaAdapter {
    pub rpc_client: Arc<RpcClient>,
}

impl SolanaAdapter {
    pub fn new(config: &ChainConfig) -> Self {
        info!("Init Solana client with url: {:?}", &config.url);
        let rpc_client = Arc::new(RpcClient::new(config.url.clone()));
        info!("Finished init Solana client");
        SolanaAdapter { rpc_client }
    }
    pub fn get_signatures_for_address(
        &self,
        address: &Pubkey,
        before_tx_signature: &Option<Signature>,
        first_slot: Option<u64>,
        end_slot: Option<u64>,
    ) -> ResultFilterTransaction {
        let mut txs: Vec<RpcConfirmedTransactionStatusWithSignature> = vec![];
        let _is_done = false;
        let client = self.rpc_client.clone();
        let config = GetConfirmedSignaturesForAddress2Config {
            before: before_tx_signature.clone(),
            until: None,
            limit: Some(LIMIT_FILTER_RESULT),
            commitment: None,
        };
        match client.get_signatures_for_address_with_config(address, config) {
            Ok(vec) => {}
            Err(_) => {}
        }
        txs.append(&mut res.unwrap_or(vec![]));

        // Fixme: Cover the case that multi addresses are in filter, now the logic is correct for filter 1 address only
        let last_tx_signature = txs
            .last()
            .map(|tx| Signature::from_str(&tx.signature).unwrap());

        // last_tx_signature.is_none: when we cannot found any result
        // txs.last().unwrap().slot < first_slot.unwrap(): when searching is out of range
        let is_done = last_tx_signature.is_none()
            || (!first_slot.is_none() && txs.last().unwrap().slot < first_slot.unwrap());

        let txs: Vec<RpcConfirmedTransactionStatusWithSignature> = txs
            .into_iter()
            .filter(|tx| {
                // Block is out of range or error
                if !tx.err.is_none() {
                    debug!("Confirmed Transaction Error: {:?}", tx.err)
                }
                (first_slot.is_some() && tx.slot < first_slot.unwrap()) || tx.err.is_none()
            })
            .collect();

        ResultFilterTransaction {
            txs,
            last_tx_signature,
            is_done,
        }
    }
}
#[derive(Clone)]
pub struct SolanaNetworkAdapter {
    pub network: String,
    pub adapter: Arc<SolanaAdapter>,
}

impl SolanaNetworkAdapter {
    pub fn new(network: String, config: &ChainConfig) -> Self {
        SolanaNetworkAdapter {
            network,
            adapter: Arc::new(SolanaAdapter::new(config)),
        }
    }
    pub fn from(network: String) -> Self {
        let config = super::SOLANA_NETWORKS
            .get(network.as_str())
            .or(super::SOLANA_NETWORKS.get("mainnet"))
            .unwrap();
        SolanaNetworkAdapter {
            network,
            adapter: Arc::new(SolanaAdapter::new(config)),
        }
    }
    pub fn get_adapter(&self) -> Arc<SolanaAdapter> {
        self.adapter.clone();
    }
}
#[derive(Clone)]
pub struct SolanaNetworkAdapters {
    pub adapters: Vec<SolanaNetworkAdapter>,
}

pub struct RuntimeAdapter {
    pub sol_adapters: Arc<SolanaNetworkAdapters>,
}
impl bc::RuntimeAdapter<Chain> for RuntimeAdapter {
    fn host_fns(&self, ds: &DataSource) -> Result<Vec<HostFn>, Error> {
        todo!()
    }
}
