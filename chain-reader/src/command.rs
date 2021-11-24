use crate::grpc_stream::StreamService;
use crate::CONFIG;
use log::{error, info};
use massbit::firehose::bstream::{stream_server::StreamServer, BlockResponse, ChainType};
use massbit::firehose::endpoints::FirehoseNetworkEndpoints;
use massbit::log::logger;
use massbit::prelude::LoggerFactory;
use massbit_common::NetworkType;
use solana_client::rpc_client::RpcClient;
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::broadcast;
use tonic::transport::Server;

#[derive(Clone, Debug)]
pub struct Config {
    pub chains: Vec<ChainConfig>,
    pub url: String,
}

impl Config {
    pub(crate) fn get_chain_config(
        &self,
        chain: &ChainType,
        network: &NetworkType,
    ) -> Option<ChainConfig> {
        for config in self.chains.iter() {
            if (&config.chain_type == chain) && (&config.network == network) {
                return Some(config.clone());
            }
        }
        return None;
    }
}

#[derive(Clone, Debug)]
pub struct ChainConfig {
    pub url: String,
    pub ws: String,
    pub start_block: Option<u64>,
    pub chain_type: ChainType,
    pub network: NetworkType,
    pub supports_eip_1898: bool,
}

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let logger = logger(true);
    // Broadcast Channel
    let mut chans: HashMap<(ChainType, NetworkType), broadcast::Sender<BlockResponse>> =
        HashMap::new();
    let mut solana_adaptors: HashMap<NetworkType, Arc<RpcClient>> = HashMap::new();
    // Spawm thread get_data
    for config in CONFIG.chains.clone().into_iter() {
        let chain_type = config.chain_type;
        let network = config.network;
        let (chan, _) = broadcast::channel(1024);
        // Clone broadcast channel
        let chan_sender = chan.clone();
        let network_clone = network.clone();
        let logger_factory = LoggerFactory::new(logger.clone());
        match chain_type {
            // Spawn Substrate get_data
            ChainType::Solana => {
                // Get Solana adapter
                let config = CONFIG.get_chain_config(&chain_type, &network).unwrap();
                let json_rpc_url = config.url.clone();
                info!("Init Solana client, url: {}", json_rpc_url);
                info!("Finished init Solana client");
                let client = Arc::new(RpcClient::new(json_rpc_url.clone()));

                solana_adaptors.insert(network_clone, client);
            }
            _ => {
                error!("Not support chain {:?}", chain_type);
            }
        }
        // add chan to chans
        chans.insert((chain_type, network), chan);
    }

    // Run StreamoutServer
    let stream_service = StreamService {
        chans,
        solana_adaptors,
    };

    let addr = CONFIG.url.parse()?;
    Server::builder()
        .add_service(StreamServer::new(stream_service))
        .serve(addr)
        .await?;

    // End
    Ok(())
}
