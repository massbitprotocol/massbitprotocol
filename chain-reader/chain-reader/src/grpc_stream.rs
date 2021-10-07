use crate::{ethereum_chain, solana_chain};
use chain_ethereum::network::{EthereumNetworkAdapter, EthereumNetworkAdapters};
use chain_ethereum::{Chain, EthereumAdapter, Transport, TriggerFilter};
use log::{error, info};
use massbit::firehose::bstream::{stream_server::Stream, BlockResponse, BlocksRequest, ChainType};
use massbit::prelude::Debug;
use massbit_common::NetworkType;
use solana_client::rpc_response::SlotInfo;
use solana_client::{pubsub_client::PubsubClient, rpc_client::RpcClient};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio::task;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

const QUEUE_BUFFER: usize = 1024;

// pub mod stream_mod {
//     tonic::include_proto!("chaindata");
// }

pub struct StreamService {
    pub chans: HashMap<(ChainType, NetworkType), broadcast::Sender<BlockResponse>>,
    pub ethereum_chains: HashMap<(ChainType, NetworkType), Arc<Chain>>,
    pub solana_adaptors: HashMap<NetworkType, (Arc<RpcClient>)>,
}

#[tonic::async_trait]
impl Stream for StreamService {
    type BlocksStream = ReceiverStream<Result<BlockResponse, Status>>;

    async fn blocks(
        &self,
        request: Request<BlocksRequest>,
    ) -> Result<Response<Self::BlocksStream>, Status> {
        info!("Request = {:?}", request);
        let chain_type: ChainType = ChainType::from_i32(request.get_ref().chain_type).unwrap();
        let network: NetworkType = request.get_ref().network.clone();
        let start_block = request.get_ref().start_block_number;
        let (tx, rx) = mpsc::channel(QUEUE_BUFFER);
        let encoded_filter: Vec<u8> = request.get_ref().filter.clone();
        match chain_type {
            ChainType::Substrate => {
                // tx, rx for out stream gRPC
                // let (tx, rx) = mpsc::channel(1024);

                // Create new channel for connect between input and output stream
                println!(
                    "chains: {:?}, chain_type: {:?}, network: {}",
                    &self.chans, chain_type, network
                );
                let sender = self.chans.get(&(chain_type, network));
                assert!(
                    sender.is_some(),
                    "Error: No channel for {:?}, check config value",
                    chain_type
                );

                let mut rx_chan = sender.unwrap().subscribe();

                tokio::spawn(async move {
                    loop {
                        // Getting generic_data
                        let generic_data = rx_chan.recv().await.unwrap();
                        // Send generic_data to queue"
                        let res = tx.send(Ok(generic_data)).await;
                        if res.is_err() {
                            error!("Cannot send data to RPC client queue, error: {:?}", res);
                        }
                    }
                });
            }
            ChainType::Solana => {
                // Spawn task
                let client = self.solana_adaptors.get(&network).unwrap().clone();
                let name = "deployment_solana".to_string();

                massbit::spawn_thread(name, move || {
                    massbit::block_on(task::unconstrained(async {
                        // Todo: add start at save block after restart
                        let resp = solana_chain::loop_get_block(
                            tx.clone(),
                            &start_block,
                            &network,
                            &client,
                        )
                        .await;
                        error!("Restart {:?} response {:?}", &chain_type, resp);
                    }))
                });
            }
            ChainType::Ethereum => {
                let name = "deployment_ethereum".to_string();
                let chain = self
                    .ethereum_chains
                    .get(&(chain_type, network.clone()))
                    .unwrap()
                    .clone();

                massbit::spawn_thread(name, move || {
                    massbit::block_on(task::unconstrained(async {
                        let filter: TriggerFilter =
                            serde_json::from_slice(&encoded_filter).unwrap_or_default();
                        let resp = ethereum_chain::loop_get_block(
                            tx.clone(),
                            &start_block,
                            &network,
                            chain,
                            filter,
                        )
                        .await;

                        error!("Stop loop_get_block, error: {:?}", resp);
                    }))
                });
            }
        }

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
