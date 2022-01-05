pub mod buffer;
pub mod runtime;
pub mod streaming;

use crate::manager::buffer::IncomingBlocks;
use indexer_orm::models::Indexer;
use massbit::ipfs_client::IpfsClient;
use massbit::slog::Logger;
use massbit_common::prelude::anyhow;
use massbit_common::prelude::diesel::r2d2::ConnectionManager;
use massbit_common::prelude::diesel::{r2d2, PgConnection};
use massbit_grpc::firehose::bstream::BlockResponse;
pub use runtime::IndexerRuntime;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tonic::Streaming;

const BUFFER_SIZE: usize = 1024;
pub struct IndexerManager {
    pub ipfs_client: Arc<IpfsClient>,
    pub connection_pool: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
    pub runtimes: HashMap<String, JoinHandle<()>>,
    //Buffer of incoming blocks by address
    pub block_buffers: HashMap<String, Arc<IncomingBlocks>>,
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
                .unwrap_or(Arc::new(IncomingBlocks::new(1024)));
            if !self.block_buffers.contains_key(address) {
                self.block_buffers.insert(address.clone(), buffer.clone());
                //Start block stream for specified address if not started
                self.start_block_stream(address, buffer.clone());
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
    fn start_block_stream(&mut self, address: &String, buffer: Arc<IncomingBlocks>) {
        let join_handle = tokio::spawn(async move { loop {} });
    }
    // async fn try_create_block_stream(&self, address: String) -> Option<Streaming<BlockResponse>> {
    //     let transaction_request = BlockRequest {
    //         indexer_hash: self.indexer.hash.clone(),
    //         start_block_number: start_block,
    //         chain_type: chain_type as i32,
    //         network: data_source.network.clone().unwrap_or(Default::default()),
    //         filter: encoded_filter,
    //     };
    //     if let Ok(channel) = Channel::from_static(CHAIN_READER_URL.as_str())
    //         .connect()
    //         .await
    //     {
    //         let timeout_channel = Timeout::new(channel, Duration::from_secs(GET_BLOCK_TIMEOUT_SEC));
    //         let mut client = StreamClient::new(timeout_channel);
    //         match client
    //             .blocks(Request::new(transaction_request.clone()))
    //             .await
    //         {
    //             Ok(res) => Some(res.into_inner()),
    //             Err(err) => {
    //                 log::error!("Create new stream with error {:?}", &err);
    //                 None
    //             }
    //         }
    //     } else {
    //         log::error!(
    //             "Cannot connect to chain reader at address {:?}",
    //             CHAIN_READER_URL.as_str()
    //         );
    //         None
    //     }
    // }
}
