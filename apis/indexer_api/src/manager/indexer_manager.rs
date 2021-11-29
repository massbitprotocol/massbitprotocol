use super::IndexerRuntime;
use crate::orm::models::Indexer;
use massbit::data::indexer::MAX_SPEC_VERSION;
use massbit::ipfs_client::IpfsClient;
use massbit::ipfs_link_resolver::LinkResolver;
use massbit::prelude::anyhow::Context;
use massbit::prelude::prost::bytes::Bytes;
use massbit::prelude::reqwest::Error;
use massbit_common::prelude::anyhow;
use massbit_common::prelude::diesel::r2d2::ConnectionManager;
use massbit_common::prelude::diesel::{
    r2d2, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl,
};
use std::collections::HashMap;

use massbit::components::store::{DeploymentId, DeploymentLocator};
use massbit::prelude::LoggerFactory;
use massbit::slog::Logger;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

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
            if let Some(mut runtime) =
                IndexerRuntime::new(indexer, ipfs_client, connection_pool, logger).await
            {
                runtime.start().await;
            }
        });
        Ok(())
    }
}
