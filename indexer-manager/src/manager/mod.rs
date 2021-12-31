pub mod runtime;
use crate::INDEXER_PROCESS_THREAD_LIMIT;
use indexer_orm::models::Indexer;
use massbit::ipfs_client::IpfsClient;
use massbit::slog::Logger;
use massbit_common::prelude::anyhow;
use massbit_common::prelude::diesel::r2d2::ConnectionManager;
use massbit_common::prelude::diesel::{r2d2, PgConnection};
pub use runtime::IndexerRuntime;
use std::sync::Arc;

pub struct IndexerManager {
    pub ipfs_client: Arc<IpfsClient>,
    pub connection_pool: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
    //pub runtimes: HashMap<String, JoinHandle<_>>,
    pub logger: Logger,
}

impl IndexerManager {
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
        let join_handle = tokio::spawn(async move {
            if let Some(mut runtime) = IndexerRuntime::new(
                indexer,
                ipfs_client,
                connection_pool,
                logger,
                INDEXER_PROCESS_THREAD_LIMIT,
            )
            .await
            {
                runtime.start().await;
            }
        });
        Ok(())
    }
}
