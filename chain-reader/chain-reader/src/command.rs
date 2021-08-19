use crate::ethereum_chain;
use crate::grpc_stream::StreamService;
use crate::solana_chain;
use crate::substrate_chain;
use crate::{
    grpc_stream::stream_mod::{streamout_server::StreamoutServer, ChainType},
    CONFIG,
};
use anyhow::Result;
use lazy_static::lazy_static;
use std::collections::HashMap;
use tokio::sync::broadcast;
use tonic::transport::Server;

lazy_static! {
    static ref HTTP_PORT: u64 = std::env::var("HTTP_PORT")
        .unwrap_or("50051".into())
        .parse::<u64>()
        .expect("invalid HTTP_PORT env var");
}

#[derive(Clone, Debug)]
pub struct Config {
    pub chains: HashMap<ChainType, ChainConfig>,
    pub url: String,
}

#[derive(Clone, Debug)]
pub struct ChainConfig {
    pub url: String,
    pub ws: String,
}

pub async fn run() -> Result<()> {
    // Broadcast Channel
    let mut chans = HashMap::new();

    // Spawm thread get_data
    for chain_type in CONFIG.chains.keys() {
        let (chan, _) = broadcast::channel(1024);
        // Clone broadcast channel
        let chan_sender = chan.clone();

        match chain_type {
            // Spawn Substrate get_data
            ChainType::Substrate => {
                // Spawn task
                tokio::spawn(async move {
                    substrate_chain::loop_get_block_and_extrinsic(chan_sender).await;
                });
                let chan_sender = chan.clone();
                // Spawn task
                tokio::spawn(async move {
                    substrate_chain::loop_get_event(chan_sender).await;
                });
                // add chan to chans
                chans.insert(ChainType::Substrate, chan);
            }
            ChainType::Solana => {
                // Spawn task
                tokio::spawn(async move {
                    solana_chain::loop_get_block(chan_sender).await;
                });
                // add chan to chans
                chans.insert(ChainType::Solana, chan);
            }
            ChainType::Ethereum => {
                // Spawn task
                tokio::spawn(async move {
                    ethereum_chain::loop_get_block(chan_sender).await;
                });
                // add chan to chans
                chans.insert(ChainType::Ethereum, chan);
            }
        }
    }

    let stream_service = StreamService { chans };
    Server::builder()
        .add_service(StreamoutServer::new(stream_service))
        .serve(format!(":{}", *HTTP_PORT).parse()?)
        .await?;

    Ok(())
}
