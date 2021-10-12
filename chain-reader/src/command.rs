use crate::grpc_stream::StreamService;
use crate::substrate_chain;
use crate::CONFIG;
use chain_ethereum::network::{EthereumNetworkAdapter, EthereumNetworkAdapters};
use chain_ethereum::{Chain, EthereumAdapter, Transport};
use log::{error, info};
use massbit::firehose::bstream::{stream_server::StreamServer, BlockResponse, ChainType};
use massbit::firehose::endpoints::FirehoseNetworkEndpoints;
use massbit::log::logger;
use massbit::prelude::LoggerFactory;
use massbit_common::NetworkType;
use solana_client::rpc_client::RpcClient;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
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

pub fn fix_one_thread_not_receive(chan: &broadcast::Sender<BlockResponse>) {
    // Todo: More clean solution for broadcast channel
    let mut rx = chan.subscribe();
    tokio::spawn(async move {
        loop {
            let _ = rx.recv().await;
        }
    });
}

async fn create_adaptor(chain_type: &ChainType, network: &NetworkType) -> EthereumAdapter {
    let logger = logger(true);
    let config = CONFIG.get_chain_config(chain_type, network).unwrap();
    let websocket_url = config.ws.clone();
    let http_url = config.url.clone();
    let supports_eip_1898 = config.supports_eip_1898;

    let (transport_event_loop, transport) = match crate::ethereum_chain::USE_WEBSOCKET {
        false => Transport::new_rpc(&http_url, Default::default()),
        true => Transport::new_ws(&websocket_url),
    };
    std::mem::forget(transport_event_loop);
    EthereumAdapter::new(
        logger,
        config.network,
        http_url.as_str(),
        transport,
        supports_eip_1898,
    )
    .await
}

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let logger = logger(true);
    // Broadcast Channel
    let mut chans: HashMap<(ChainType, NetworkType), broadcast::Sender<BlockResponse>> =
        HashMap::new();
    let mut ethereum_chains: HashMap<(ChainType, NetworkType), Arc<Chain>> = HashMap::new();
    let mut solana_adaptors: HashMap<NetworkType, Arc<RpcClient>> = HashMap::new();
    // Spawm thread get_data
    for config in CONFIG.chains.clone().into_iter() {
        let chain_type = config.chain_type;
        let network = config.network;
        let (chan, _) = broadcast::channel(1024);
        // Clone broadcast channel
        let chan_sender = chan.clone();
        fix_one_thread_not_receive(&chan_sender);
        let network_clone = network.clone();
        let logger_factory = LoggerFactory::new(logger.clone());
        match chain_type {
            // Spawn Substrate get_data
            ChainType::Substrate => {
                // Spawn task
                tokio::spawn(async move {
                    //fix_one_thread_not_receive(&chan_sender);
                    // Todo: add start at save block after restart
                    let mut count = 1;
                    loop {
                        let resp =
                            substrate_chain::loop_get_block_and_extrinsic(chan_sender.clone())
                                .await;
                        error!(
                            "Restart {:?} response {:?}, {} time",
                            &chain_type, resp, count
                        );
                        sleep(Duration::from_secs(1));
                        count = count + 1;
                    }
                });
                let chan_sender = chan.clone();
                // Spawn task
                tokio::spawn(async move {
                    //fix_one_thread_not_receive(&chan_sender);
                    let mut count = 1;
                    loop {
                        let resp = substrate_chain::loop_get_event(chan_sender.clone()).await;
                        error!(
                            "Restart {:?} response {:?}, {} time",
                            &chain_type, resp, count
                        );
                        sleep(Duration::from_secs(1));
                        count = count + 1;
                    }
                });
                // add chan to chans
                //chans.insert((ChainType::Substrate,), chan);
            }
            ChainType::Solana => {
                // Get Solana adapter
                let config = CONFIG.get_chain_config(&chain_type, &network).unwrap();
                let json_rpc_url = config.url.clone();
                info!("Init Solana client, url: {}", json_rpc_url);
                info!("Finished init Solana client");
                let client = Arc::new(RpcClient::new(json_rpc_url.clone()));

                solana_adaptors.insert(network_clone, client);

                // Spawn task
                // tokio::spawn(async move {
                //     // Todo: add start at save block after restart
                //     let mut count = 1;
                //     loop {
                //         let resp =
                //             solana_chain::loop_get_block(chan_sender.clone(), &network_clone).await;
                //         error!(
                //             "Restart {:?} response {:?}, {} time",
                //             &chain_type, resp, count
                //         );
                //         sleep(Duration::from_secs(1));
                //         count = count + 1;
                //     }
                // });
                // add chan to chans
                //chans.insert(ChainType::Solana, chan);
            }
            ChainType::Ethereum => {
                let chain = Chain::new(
                    logger_factory,
                    network.clone(),
                    EthereumNetworkAdapters {
                        adapters: vec![EthereumNetworkAdapter {
                            adapter: Arc::new(create_adaptor(&chain_type, &network).await),
                        }],
                    },
                    FirehoseNetworkEndpoints { endpoints: vec![] },
                );
                ethereum_chains.insert(
                    (ChainType::Ethereum, network_clone.clone()),
                    Arc::new(chain),
                );
            }
        }
        // add chan to chans
        chans.insert((chain_type, network), chan);
    }

    // Run StreamoutServer
    let stream_service = StreamService {
        chans,
        ethereum_chains,
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
