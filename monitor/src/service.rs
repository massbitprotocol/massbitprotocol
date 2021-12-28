use indexer_orm::models::Indexer;
use massbit_common::prelude::diesel::{
    r2d2::{self, ConnectionManager},
    ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl,
};
use massbit_common::prelude::r2d2::PooledConnection;
use massbit_common::prelude::{anyhow, log};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct Monitor {
    connection_pool: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
    period: u64,
}

impl Monitor {
    pub fn new(
        connection_pool: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
        period: u64,
    ) -> Self {
        Self {
            connection_pool,
            period,
        }
    }
    pub fn get_connection(
        &self,
    ) -> Result<
        PooledConnection<ConnectionManager<PgConnection>>,
        massbit_common::prelude::r2d2::Error,
    > {
        self.connection_pool.get()
    }
    pub fn start(&self) {
        loop {
            let start = Instant::now();
            //Start stopped indexers
            let stopped_indexer = self.get_stopped_indexer();
            for indexer in stopped_indexer.iter() {
                if let Err(err) = self.start_indexer(indexer) {
                    log::error!("{:?}", &err);
                }
            }
            if start.elapsed().as_millis() < self.period as u128 {
                sleep(Duration::from_millis(
                    self.period - start.elapsed().as_millis() as u64,
                ));
            }
        }
    }
    /// Get all stopped indexer from db and start them
    /// Stopped indexers are defined as ....
    /// Todo: Clarify this definition
    fn get_stopped_indexer(&self) -> Vec<Indexer> {
        vec![]
    }
    /// Call api start indexer in indexer-manager to start an indexer runtime
    /// like a request deploy from indexer-api
    fn start_indexer(&self, indexer: &Indexer) -> Result<(), anyhow::Error> {
        Ok(())
    }
}
