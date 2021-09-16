use crate::ethereum_chain;
use crate::grpc_stream::StreamService;
use crate::solana_chain;
use crate::substrate_chain;
use crate::{
    grpc_stream::stream_mod::{streamout_server::StreamoutServer, ChainType, GenericDataProto},
    CONFIG,
};
use graph::semver::Op;
use log::error;
use std::collections::HashMap;
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

pub type NetworkType = String;

#[derive(Clone, Debug)]
pub struct ChainConfig {
    pub url: String,
    pub ws: String,
    pub start_block: Option<u64>,
    pub chain_type: ChainType,
    pub network: NetworkType,
}

pub fn fix_one_thread_not_receive(chan: &broadcast::Sender<GenericDataProto>) {
    // Todo: More clean solution for broadcast channel
    let mut rx = chan.subscribe();
    tokio::spawn(async move {
        loop {
            let _ = rx.recv().await;
        }
    });
}

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Broadcast Channel
    let mut chans: HashMap<(ChainType, NetworkType), broadcast::Sender<GenericDataProto>> =
        HashMap::new();

    // Spawm thread get_data
    for config in CONFIG.chains.clone().into_iter() {
        let chain_type = config.chain_type;
        let network = config.network;
        let (chan, _) = broadcast::channel(1024);
        // Clone broadcast channel
        let chan_sender = chan.clone();
        fix_one_thread_not_receive(&chan_sender);
        let network_clone = network.clone();
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
                // Spawn task
                //fix_one_thread_not_receive(&chan_sender);
                tokio::spawn(async move {
                    // Todo: add start at save block after restart
                    let mut count = 1;
                    loop {
                        let resp =
                            solana_chain::loop_get_block(chan_sender.clone(), &network_clone).await;
                        error!(
                            "Restart {:?} response {:?}, {} time",
                            &chain_type, resp, count
                        );
                        sleep(Duration::from_secs(1));
                        count = count + 1;
                    }
                });
                // add chan to chans
                //chans.insert(ChainType::Solana, chan);
            }
            ChainType::Ethereum => {
                // Spawn task
                tokio::spawn(async move {
                    // fix_one_thread_not_receive(&chan_sender);
                    let mut count = 1;
                    let mut got_block_number = CONFIG
                        .get_chain_config(&chain_type, &network_clone)
                        .unwrap()
                        .start_block;
                    loop {
                        let resp = ethereum_chain::loop_get_block(
                            chan_sender.clone(),
                            &mut got_block_number,
                            network_clone.clone(),
                        )
                        .await;
                        error!(
                            "Restart {:?} response {:?}, at block {:?}, {} time",
                            &chain_type, resp, &got_block_number, count
                        );
                        sleep(Duration::from_secs(1));
                        count = count + 1;
                    }
                });
                // add chan to chans
                //chans.insert(ChainType::Ethereum, chan);
            }
        }
        // add chan to chans
        chans.insert((chain_type, network), chan);
    }

    // Run StreamoutServer
    let stream_service = StreamService { chans: chans };

    let addr = CONFIG.url.parse()?;
    Server::builder()
        .add_service(StreamoutServer::new(stream_service))
        .serve(addr)
        .await?;

    // End
    Ok(())
}
