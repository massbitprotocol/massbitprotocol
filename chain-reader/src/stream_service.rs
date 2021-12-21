//use crate::command::Config;
use crate::indexer_broadcast::IndexerBroadcast;
use crate::solana_chain;
use crate::solana_chain_adapter::ChainAdapter;
use chain_ethereum::{Chain, TriggerFilter};
use chain_solana::adapter::{SolanaNetworkAdapter, SolanaNetworkAdapters};
use chain_solana::types::{ChainConfig, ConfirmedBlockWithSlot};
use chain_solana::SOLANA_NETWORKS;
use log::{error, info};
use massbit::prelude::tokio::sync::mpsc::Sender;
use massbit_chain_solana::data_type::SolanaFilter;
use massbit_common::prelude::tokio::sync::RwLock;
use massbit_common::prelude::tokio::time::{sleep, Duration};
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
use web3::types::BlockId::Hash;

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
            let mut service = NetworkService::new(network);
            &service.init();
            services.insert(network.clone(), service);
        }
        if let Some(service) = services.get_mut(network) {
            service.register_indexer(request.get_ref(), tx);
        };
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

struct NetworkService {
    network: String,
    chain_adapters: Arc<Mutex<SolanaNetworkAdapters>>,
    broadcaster: Arc<Mutex<IndexerBroadcast>>,
}

impl NetworkService {
    fn new(network: &str) -> Self {
        let (tx, rx) = mpsc::channel(QUEUE_BUFFER);
        //let chain_adapter = Arc::new(Mutex::new(ChainAdapter::new(config, tx)));
        let broadcaster = Arc::new(Mutex::new(IndexerBroadcast::new(rx)));
        NetworkService {
            network: network.to_string(),
            chain_adapters: Arc::new(Mutex::new(SolanaNetworkAdapters::new(network, tx))),
            broadcaster,
        }
    }
    fn init(&mut self) {
        /// chain reader thread
        let mut chain_adapters = self.chain_adapters.clone();
        let name = format!("{:?}_reader", &self.network);
        massbit::spawn_thread(name, move || {
            massbit::block_on(task::unconstrained(async {
                chain_adapters.lock().unwrap().start().await;
            }))
        });
        let mut broadcaster = self.broadcaster.clone();
        let name = format!("{:?}_broadcaster", &self.network);
        massbit::spawn_thread(name, move || {
            massbit::block_on(task::unconstrained(async {
                loop {
                    ///Try get incoming block from chain adapter
                    let success = broadcaster.lock().unwrap().try_recv().await;
                    if !success {
                        sleep(Duration::from_millis(100)).await;
                        print!(".");
                    }
                }
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
