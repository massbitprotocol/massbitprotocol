use crate::solana_chain;
use log::{error, info};
use massbit::firehose::bstream::{stream_server::Stream, BlockRequest, BlockResponse, ChainType};
use massbit_chain_solana::data_type::SolanaFilter;
use massbit_common::NetworkType;
use solana_client::rpc_client::RpcClient;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio::task;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

const QUEUE_BUFFER: usize = 1024;

pub struct StreamService {
    pub chans: HashMap<(ChainType, NetworkType), broadcast::Sender<BlockResponse>>,
    pub solana_adaptors: HashMap<NetworkType, Arc<RpcClient>>,
}

#[tonic::async_trait]
impl Stream for StreamService {
    type BlocksStream = ReceiverStream<Result<BlockResponse, Status>>;
    async fn blocks(
        &self,
        request: Request<BlockRequest>,
    ) -> Result<Response<Self::BlocksStream>, Status> {
        info!("Request = {:?}", request);
        let chain_type: ChainType = ChainType::from_i32(request.get_ref().chain_type).unwrap();
        let network: NetworkType = request.get_ref().network.clone();
        let start_block = request.get_ref().start_block_number;
        let (tx, rx) = mpsc::channel(QUEUE_BUFFER);
        let encoded_filter: Vec<u8> = request.get_ref().filter.clone();
        match chain_type {
            ChainType::Solana => {
                // Decode filter
                let filter: SolanaFilter =
                    serde_json::from_slice(&encoded_filter).unwrap_or_default();

                let client = self.solana_adaptors.get(&network).unwrap().clone();
                let name = "deployment_solana".to_string();

                // Spawn task
                massbit::spawn_thread(name, move || {
                    massbit::block_on(task::unconstrained(async {
                        // Todo: add start at save block after restart
                        let resp = solana_chain::loop_get_block(
                            tx.clone(),
                            &start_block,
                            &network,
                            &client,
                            &filter,
                        )
                        .await;
                        error!("{:?} chain loop_get_block stop: {:?}", &chain_type, resp);
                    }))
                });
            }
            _ => {
                error!("Not support chain {:?}", chain_type);
            }
        }

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
