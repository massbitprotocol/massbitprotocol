use crate::connection_pool::ConnectionPool;
use crate::deployment;
use crate::primary::Site;
use crate::relational::{Layout, LayoutCache};
use crate::relational_queries::FromEntityData;
use lru_time_cache::LruCache;
use massbit_common::cheap_clone::CheapClone;
use massbit_common::prelude::anyhow::{anyhow, Error};
use massbit_common::prelude::diesel::{
    r2d2::{self, ConnectionManager, PooledConnection},
    Connection, PgConnection,
};
use massbit_common::prelude::slog::{debug, o};
use massbit_common::prelude::{lazy_static::lazy_static, slog::Logger, tokio};
use massbit_data::indexer::{DeploymentHash, DeploymentState};
use massbit_data::prelude::{EntityQuery, QueryExecutionError, StoreError};
use massbit_data::schema::api::api_schema;
use massbit_data::schema::{ApiSchema, Schema};
use massbit_data::store::{BlockNumber, BlockPtr, PoolWaitStats};
use massbit_data::utils::futures::{CancelHandle, CancelableError};
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

lazy_static! {
    /// `STATS_REFRESH_INTERVAL` is how long statistics that
    /// influence query execution are cached in memory (in seconds) before
    /// they are reloaded from the database. Defaults to 300s (5 minutes).
    static ref STATS_REFRESH_INTERVAL: Duration = {
        env::var("STATS_REFRESH_INTERVAL")
        .ok()
        .map(|s| {
            let secs = u64::from_str(&s).unwrap_or_else(|_| {
                panic!("STATS_REFRESH_INTERVAL must be a number, but is `{}`", s)
            });
            Duration::from_secs(secs)
        }).unwrap_or(Duration::from_secs(300))
    };
}

/// When connected to read replicas, this allows choosing which DB server to use for an operation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ReplicaId {
    /// The main server has write and read access.
    Main,

    /// A read replica identified by its index.
    ReadOnly(usize),
}

/// Commonly needed information about a subgraph that we cache in
/// `Store.subgraph_cache`. Only immutable subgraph data can be cached this
/// way as the cache lives for the lifetime of the `Store` object
#[derive(Clone)]
pub(crate) struct IndexerInfo {
    /// The schema as supplied by the user
    pub(crate) input: Arc<Schema>,
    /// The schema we derive from `input` with `graphql::schema::api::api_schema`
    pub(crate) api: Arc<ApiSchema>,
    /// The block number at which this indexer was grafted onto
    /// another one. We do not allow reverting past this block
    pub(crate) graft_block: Option<BlockNumber>,
    //pub(crate) description: Option<String>,
    //pub(crate) repository: Option<String>,
}

pub struct StoreInner {
    logger: Logger,
    conn: ConnectionPool,
    read_only_pools: Vec<ConnectionPool>,

    /// A cache of commonly needed data about a indexer.
    indexer_cache: Mutex<LruCache<DeploymentHash, IndexerInfo>>,
    /// A cache for the layout metadata for subgraphs. The Store just
    /// hosts this because it lives long enough, but it is managed from
    /// the entities module
    pub(crate) layout_cache: LayoutCache,
}
#[derive(Clone)]
pub struct DeploymentStore(Arc<StoreInner>);

impl CheapClone for DeploymentStore {}

impl Deref for DeploymentStore {
    type Target = StoreInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DeploymentStore {
    pub fn new(
        logger: &Logger,
        pool: ConnectionPool,
        read_only_pools: Vec<ConnectionPool>,
        mut pool_weights: Vec<usize>,
    ) -> Self {
        // Create a store-specific logger
        let logger = logger.new(o!("component" => "Store"));

        // Create a list of replicas with repetitions according to the weights
        // and shuffle the resulting list. Any missing weights in the list
        // default to 1
        pool_weights.resize(read_only_pools.len() + 1, 1);
        let mut replica_order: Vec<_> = pool_weights
            .iter()
            .enumerate()
            .map(|(i, weight)| {
                let replica = if i == 0 {
                    ReplicaId::Main
                } else {
                    ReplicaId::ReadOnly(i - 1)
                };
                vec![replica; *weight]
            })
            .flatten()
            .collect();
        let mut rng = thread_rng();
        replica_order.shuffle(&mut rng);
        debug!(logger, "Using postgres host order {:?}", replica_order);
        let store = StoreInner {
            logger: logger.clone(),
            conn: pool,
            read_only_pools,
            indexer_cache: Mutex::new(LruCache::with_capacity(100)),
            layout_cache: LayoutCache::new(*STATS_REFRESH_INTERVAL),
        };
        DeploymentStore(Arc::new(store))
    }
    fn get_conn(&self) -> Result<PooledConnection<ConnectionManager<PgConnection>>, Error> {
        self.conn.get_with_timeout_warning(&self.logger)
    }
    fn read_only_conn(
        &self,
        idx: usize,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, Error> {
        self.read_only_pools[idx].get().map_err(Error::from)
    }
    pub(crate) fn get_replica_conn(
        &self,
        replica: ReplicaId,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, Error> {
        let conn = match replica {
            ReplicaId::Main => self.get_conn()?,
            ReplicaId::ReadOnly(idx) => self.read_only_conn(idx)?,
        };
        Ok(conn)
    }
    pub(crate) async fn query_permit(
        &self,
        replica: ReplicaId,
    ) -> tokio::sync::OwnedSemaphorePermit {
        let pool = match replica {
            ReplicaId::Main => &self.conn,
            ReplicaId::ReadOnly(idx) => &self.read_only_pools[idx],
        };
        pool.query_permit().await
    }

    pub(crate) fn wait_stats(&self, replica: ReplicaId) -> &PoolWaitStats {
        match replica {
            ReplicaId::Main => &self.conn.wait_stats,
            ReplicaId::ReadOnly(idx) => &self.read_only_pools[idx].wait_stats,
        }
    }
    pub(crate) async fn with_conn<T: Send + 'static>(
        &self,
        f: impl 'static
            + Send
            + FnOnce(
                &PooledConnection<ConnectionManager<PgConnection>>,
                &CancelHandle,
            ) -> Result<T, CancelableError<StoreError>>,
    ) -> Result<T, StoreError> {
        self.conn.with_conn(f).await
    }
    pub(crate) fn execute_query<T: FromEntityData>(
        &self,
        conn: &PgConnection,
        site: Arc<Site>,
        query: EntityQuery,
    ) -> Result<Vec<T>, QueryExecutionError> {
        let layout = self.layout(conn, site)?;

        let logger = query.logger.unwrap_or(self.logger.clone());
        layout.query(
            &logger,
            conn,
            query.collection,
            query.filter,
            query.order,
            query.range,
            query.block,
            query.query_id,
        )
    }
    /// Return the layout for a deployment. Since constructing a `Layout`
    /// object takes a bit of computation, we cache layout objects that do
    /// not have a pending migration in the Store, i.e., for the lifetime of
    /// the Store. Layout objects with a pending migration can not be
    /// cached for longer than a transaction since they might change
    /// without us knowing
    pub(crate) fn layout(
        &self,
        conn: &PgConnection,
        site: Arc<Site>,
    ) -> Result<Arc<Layout>, StoreError> {
        self.layout_cache.get(&self.logger, conn, site)
    }

    pub(crate) async fn exists_and_synced(&self, id: DeploymentHash) -> Result<bool, StoreError> {
        self.with_conn(move |conn, _| {
            conn.transaction(|| deployment::exists_and_synced(&conn, &id))
                .map_err(Into::into)
        })
        .await
    }
    pub(crate) fn block_ptr(&self, site: &Site) -> Result<Option<BlockPtr>, Error> {
        let conn = self.get_conn()?;
        Self::block_ptr_with_conn(&site.deployment, &conn)
    }
    fn block_ptr_with_conn(
        indexer_hash: &DeploymentHash,
        conn: &PgConnection,
    ) -> Result<Option<BlockPtr>, Error> {
        deployment::block_ptr(&conn, indexer_hash).map_err(|err| anyhow!("{:?}", &err))
    }

    pub(crate) async fn deployment_state_from_id(
        &self,
        id: DeploymentHash,
    ) -> Result<DeploymentState, StoreError> {
        self.with_conn(|conn, _| deployment::state(&conn, id).map_err(|e| e.into()))
            .await
    }

    fn indexer_info_with_conn(
        &self,
        conn: &PgConnection,
        site: &Site,
    ) -> Result<IndexerInfo, StoreError> {
        if let Some(info) = self.indexer_cache.lock().unwrap().get(&site.deployment) {
            return Ok(info.clone());
        }

        let input_schema = deployment::manifest_info(&conn, site)?;

        let graft_block =
            deployment::graft_point(&conn, &site.deployment)?.map(|(_, ptr)| ptr.number as i64);

        //let features = deployment::features(&conn, site)?;
        //Not applied
        let features = BTreeSet::default();
        // Generate an API schema for the subgraph and make sure all types in the
        // API schema have a @subgraphId directive as well
        let mut schema = input_schema.clone();
        schema.document =
            api_schema(&schema.document, &features).map_err(|e| StoreError::Unknown(e.into()))?;
        schema.add_indexer_id_directives(site.deployment.clone());

        let info = IndexerInfo {
            input: Arc::new(input_schema),
            api: Arc::new(ApiSchema::from_api_schema(schema)?),
            graft_block,
            //description,
            //repository,
        };

        // Insert the schema into the cache.
        let mut cache = self.indexer_cache.lock().unwrap();
        cache.insert(site.deployment.clone(), info);

        Ok(cache.get(&site.deployment).unwrap().clone())
    }

    pub(crate) fn indexer_info(&self, site: &Site) -> Result<IndexerInfo, StoreError> {
        if let Some(info) = self.indexer_cache.lock().unwrap().get(&site.deployment) {
            return Ok(info.clone());
        }

        let conn = self.get_conn()?;
        self.indexer_info_with_conn(&conn, site)
    }
    pub(crate) fn replica_for_query(
        &self,
        for_subscription: bool,
    ) -> Result<ReplicaId, StoreError> {
        use std::sync::atomic::Ordering;

        // let replica_id = match for_subscription {
        //     // Pick a weighted ReplicaId. `replica_order` contains a list of
        //     // replicas with repetitions according to their weight
        //     false => {
        //         let weights_count = self.replica_order.len();
        //         let index =
        //             self.conn_round_robin_counter.fetch_add(1, Ordering::SeqCst) % weights_count;
        //         *self.replica_order.get(index).unwrap()
        //     }
        //     // Subscriptions always go to the main replica.
        //     true => ReplicaId::Main,
        // };
        //Todo: not support replica yet
        let replica_id = ReplicaId::Main;
        Ok(replica_id)
    }
}
