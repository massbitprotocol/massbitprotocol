pub mod buffer;
pub mod runtime;
pub mod streaming;

use crate::manager::buffer::IncomingBlocks;
use crate::manager::streaming::BlockStream;
use indexer_orm::models::Indexer;
use massbit::ipfs_client::IpfsClient;
use massbit::slog::Logger;
use massbit_common::prelude::anyhow;
use massbit_common::prelude::diesel::r2d2::ConnectionManager;
use massbit_common::prelude::diesel::{r2d2, PgConnection};
use massbit_grpc::firehose::bstream::BlockResponse;
pub use runtime::IndexerRuntime;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use tokio::task::JoinHandle;
use tonic::Streaming;

const BUFFER_SIZE: usize = 1024;
pub struct IndexerManager {
    pub ipfs_client: Arc<IpfsClient>,
    pub connection_pool: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
    pub runtimes: HashMap<String, JoinHandle<()>>,
    //Buffer of incoming blocks by address
    pub block_buffers: HashMap<String, Arc<Mutex<IncomingBlocks>>>,
    pub logger: Logger,
}

impl IndexerManager {
    pub fn new(
        ipfs_client: Arc<IpfsClient>,
        connection_pool: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
        logger: Logger,
    ) -> Self {
        Self {
            ipfs_client,
            connection_pool,
            runtimes: Default::default(),
            block_buffers: Default::default(),
            logger,
        }
    }
    pub async fn start_indexers(&mut self, indexers: &Vec<Indexer>) {
        for indexer in indexers {
            self.start_indexer(indexer.clone()).await;
        }
    }
    pub async fn start_indexer(&mut self, indexer: Indexer) -> Result<(), anyhow::Error> {
        log::info!("Start {:?}", &indexer);
        let hash = indexer.hash.clone();
        let connection_pool = self.connection_pool.clone();
        let logger = self.logger.clone();
        let ipfs_client = self.ipfs_client.clone();
        if let Some(address) = indexer.address.as_ref() {
            let buffer = self
                .block_buffers
                .get(address)
                .and_then(|arc| Some(arc.clone()))
                .unwrap_or(Arc::new(Mutex::new(IncomingBlocks::new(1024))));
            if !self.block_buffers.contains_key(address) {
                self.block_buffers.insert(address.clone(), buffer.clone());
                //Start block stream for specified address if not started
                let network = indexer
                    .network
                    .as_ref()
                    .and_then(|network| Some(network.clone()))
                    .unwrap_or_default();
                self.start_block_stream(network, address.clone(), buffer.clone())
                    .await;
            }

            let join_handle = tokio::spawn(async move {
                if let Some(mut runtime) =
                    IndexerRuntime::new(indexer, ipfs_client, connection_pool, buffer, logger).await
                {
                    runtime.start().await;
                }
            });
            self.runtimes.insert(hash, join_handle);
        }

        Ok(())
    }
    async fn start_block_stream(
        &mut self,
        network: String,
        address: String,
        buffer: Arc<Mutex<IncomingBlocks>>,
    ) -> Result<(), anyhow::Error> {
        let join_handle = tokio::spawn(async move {
            let block_stream =
                BlockStream::new(network.to_string(), address.clone(), buffer.clone());
            block_stream.start().await;
        });
        Ok(())
    }
}
