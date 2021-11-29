use crate::command::{ChainConfig, Config};
use crate::indexer_broadcast::IndexerBroadcast;
use crate::solana_chain_adapter::ChainAdapter;
use crate::SOLANA_NETWORKS;
use crate::{ethereum_chain, solana_chain};
use chain_ethereum::{Chain, TriggerFilter};
use chain_solana::types::ConfirmedBlockWithSlot;
use log::{error, info};
use massbit::prelude::tokio::sync::mpsc::Sender;
use massbit_chain_solana::data_type::SolanaFilter;
use massbit_common::prelude::tokio::sync::RwLock;
use massbit_common::NetworkType;
use massbit_grpc::firehose::bstream::{
    stream_server::Stream, BlockRequest, BlockResponse, ChainType,
};
use solana_client::rpc_client::RpcClient;
use solana_transaction_status::ConfirmedBlock;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, mpsc};
use tokio::task;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use web3::api::Net;

const QUEUE_BUFFER: usize = 1024;

pub struct StreamService {
    network_services: RwLock<HashMap<String, NetworkService>>,
}

impl StreamService {
    pub fn new() -> Self {
        StreamService {
            network_services: Default::default(),
        }
    }
}
#[tonic::async_trait]
impl Stream for StreamService {
    type BlocksStream = ReceiverStream<Result<BlockResponse, Status>>;
    async fn blocks(
        &self,
        request: Request<BlockRequest>,
    ) -> Result<Response<Self::BlocksStream>, Status> {
        info!("Request = {:?}", &request);
        let (tx, rx) = mpsc::channel(QUEUE_BUFFER);
        let network = &request.get_ref().network;
        let mut services = self.network_services.write().await;
        if !services.contains_key(network) {
            if let Some(config) = SOLANA_NETWORKS.get(network) {
                let mut service = NetworkService::new(network, config);
                &service.init();
                services.insert(network.clone(), service);
            }
        }
        if let Some(service) = services.get_mut(network) {
            service.register_indexer(request.get_ref(), tx);
        };
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

struct NetworkService {
    network: String,
    chain_adapter: Arc<Mutex<ChainAdapter>>,
    broadcaster: Arc<Mutex<IndexerBroadcast>>,
}

impl NetworkService {
    fn new(network: &String, config: &ChainConfig) -> Self {
        let (tx, rx) = mpsc::channel(QUEUE_BUFFER);
        let chain_adapter = Arc::new(Mutex::new(ChainAdapter::new(config, tx)));
        let broadcaster = Arc::new(Mutex::new(IndexerBroadcast::new(rx)));
        NetworkService {
            network: network.to_string(),
            chain_adapter,
            broadcaster,
        }
    }
    fn init(&mut self) {
        /// chain reader thread
        let mut chain_adapter = self.chain_adapter.clone();
        let name = format!("{:?}_reader", &self.network);
        massbit::spawn_thread(name, move || {
            massbit::block_on(task::unconstrained(async {
                chain_adapter.lock().unwrap().start().await;
            }))
        });
        let mut broadcaster = self.broadcaster.clone();
        let name = format!("{:?}_broadcaster", &self.network);
        massbit::spawn_thread(name, move || {
            massbit::block_on(task::unconstrained(async {
                broadcaster.lock().unwrap().start().await;
            }))
        });
    }
    fn register_indexer(
        &mut self,
        request: &BlockRequest,
        indexer_sender: Sender<Result<BlockResponse, Status>>,
    ) {
        self.broadcaster.lock().unwrap().register_indexer(
            &request.indexer_hash,
            &request.filter,
            indexer_sender,
        );
    }
}

pub enum BlockInfo {
    BlockSlot(u64),
    ConfirmBlockWithSlot(ConfirmedBlockWithSlot),
}

impl From<u64> for BlockInfo {
    fn from(slot: u64) -> Self {
        BlockInfo::BlockSlot(slot)
    }
}

impl From<(u64, ConfirmedBlock)> for BlockInfo {
    fn from(val: (u64, ConfirmedBlock)) -> Self {
        let (slot, block) = val;
        BlockInfo::ConfirmBlockWithSlot(ConfirmedBlockWithSlot {
            block_slot: slot,
            block: Some(block),
        })
    }
}
