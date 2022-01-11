use crate::manager::buffer::IncomingBlocks;
use crate::{CHAIN_READER_URL, GET_BLOCK_TIMEOUT_SEC, GET_STREAM_TIMEOUT_SEC};
use chain_solana::types::SolanaFilter;
use massbit_common::prelude::{serde_json, uuid};
use massbit_grpc::firehose::bstream::stream_client::StreamClient;
use massbit_grpc::firehose::bstream::{BlockRequest, BlockResponse, ChainType};
use massbit_solana_sdk::types::SolanaBlock;
use std::error::Error;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use tonic::transport::Channel;
use tonic::{Request, Streaming};
use tower::timeout::Timeout;

pub struct BlockStream {
    network: String,
    address: String,
    buffer: Arc<IncomingBlocks>,
}

impl BlockStream {
    pub fn new(network: String, address: String, buffer: Arc<IncomingBlocks>) -> Self {
        Self {
            network,
            address,
            buffer,
        }
    }
    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        let mut opt_stream: Option<Streaming<BlockResponse>> = None;
        loop {
            match opt_stream {
                None => {
                    opt_stream = self.try_create_block_stream().await;
                    if opt_stream.is_none() {
                        //Sleep for a while and reconnect
                        sleep(Duration::from_secs(GET_STREAM_TIMEOUT_SEC)).await;
                    }
                }
                Some(ref mut stream) => {
                    let response =
                        timeout(Duration::from_secs(GET_BLOCK_TIMEOUT_SEC), stream.message()).await;
                    match response {
                        Ok(Ok(res)) => {
                            if let Some(mut data) = res {
                                let now = Instant::now();
                                let blocks: Vec<SolanaBlock> =
                                    serde_json::from_slice(&mut data.payload).unwrap();
                                log::info!(
                                    "Deserialization time of {:?} blocks: {:?}",
                                    blocks.len(),
                                    now.elapsed()
                                );
                                if blocks.len() > 0 {
                                    self.buffer.append_blocks(blocks);
                                }
                            }
                        }
                        _ => {
                            log::info!(
                                "Error while get message from reader stream {:?}. Recreate stream",
                                &response
                            );
                            opt_stream = None;
                        }
                    }
                }
            }
        }
        Ok(())
    }
    async fn try_create_block_stream(&self) -> Option<Streaming<BlockResponse>> {
        let filter = SolanaFilter::new(vec![self.address.as_str()]);
        let encoded_filter = serde_json::to_vec(&filter).unwrap();
        log::info!(
            "Create new blocks stream: address {:?}, network {:?}.",
            &self.address,
            &self.network
        );
        //Generate random uuid
        let uuid = uuid::Uuid::new_v4().to_string();
        let transaction_request = BlockRequest {
            indexer_hash: uuid,
            start_block_number: None,
            chain_type: ChainType::Solana as i32,
            network: self.network.clone(),
            filter: encoded_filter,
        };
        if let Ok(channel) = Channel::from_static(CHAIN_READER_URL.as_str())
            .connect()
            .await
        {
            let timeout_channel = Timeout::new(channel, Duration::from_secs(GET_BLOCK_TIMEOUT_SEC));
            let mut client = StreamClient::new(timeout_channel);
            match client
                .blocks(Request::new(transaction_request.clone()))
                .await
            {
                Ok(res) => Some(res.into_inner()),
                Err(err) => {
                    log::error!("Create new stream with error {:?}", &err);
                    None
                }
            }
        } else {
            log::error!(
                "Cannot connect to chain reader at address {:?}",
                CHAIN_READER_URL.as_str()
            );
            None
        }
    }
}
