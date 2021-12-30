//use crate::command::Config;
use crate::indexer_broadcast::IndexerBroadcast;
use crate::solana_chain;
use crate::solana_chain_adapter::ChainAdapter;
use chain_ethereum::{Chain, TriggerFilter};
use chain_solana::adapter::{SolanaNetworkAdapter, SolanaNetworkAdapters};
use chain_solana::storage::{BlockStorage, LevelDBStorage};

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
    cache_db_path: Option<String>,
}

impl StreamService {
    pub fn new(cache_db_path: Option<String>) -> Self {
        StreamService {
            network_services: Default::default(),
            cache_db_path,
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
            let leveldb_storage = self.cache_db_path.as_ref().and_then(|path| {
                Some(Arc::new(Box::new(LevelDBStorage::new(path.as_str()))
                    as Box<dyn BlockStorage + Sync + Send>))
            });
            let mut service = NetworkService::new(network, leveldb_storage);
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
    fn new(network: &str, storage: Option<Arc<Box<dyn BlockStorage + Sync + Send>>>) -> Self {
        let (tx, rx) = mpsc::channel(QUEUE_BUFFER);
        //let chain_adapter = Arc::new(Mutex::new(ChainAdapter::new(config, tx)));
        let broadcaster = Arc::new(Mutex::new(IndexerBroadcast::new(rx)));
        NetworkService {
            network: network.to_string(),
            chain_adapters: Arc::new(Mutex::new(SolanaNetworkAdapters::new(
                network,
                storage,
                Some(tx),
            ))),
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
