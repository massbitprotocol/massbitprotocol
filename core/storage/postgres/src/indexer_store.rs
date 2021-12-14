use crate::deployment_store::{DeploymentStore, ReplicaId};
use crate::primary::UnusedDeployment;
use crate::{
    connection_pool::ConnectionPool,
    primary,
    primary::{DeploymentId, Site},
    relational::Layout,
};
use diesel::{
    pg::Pg,
    serialize::Output,
    sql_types::Text,
    types::{FromSql, ToSql},
    PgConnection,
};
use massbit_common::cheap_clone::CheapClone;
use massbit_common::prelude::anyhow::anyhow;
use massbit_common::prelude::diesel::r2d2::{self, ConnectionManager};
use massbit_common::prelude::futures03::future::join_all;
use massbit_common::prelude::slog::{o, Logger};
use massbit_common::prelude::tokio::sync::OwnedSemaphorePermit;
use massbit_common::prelude::{anyhow::Error, async_trait::async_trait, lazy_static::lazy_static};
use massbit_data::indexer::{DeploymentHash, DeploymentLocator, DeploymentState, NodeId};
use massbit_data::metrics::stopwatch::StopwatchMetrics;
use massbit_data::prelude::q::Value;
use massbit_data::prelude::{QueryExecutionError, QueryTarget, StoreError};
use massbit_data::schema::{ApiSchema, Schema};
use massbit_data::store::chain::{BlockNumber, BlockPtr};
use massbit_data::store::entity::EntityOrder::Default;
use massbit_data::store::entity::{Entity, EntityKey, EntityModification, EntityQuery, EntityType};
use massbit_data::store::{
    PoolWaitStats, QueryStore as QueryStoreTrait, QueryStore, QueryStoreManager, StoreEvent,
};
use massbit_data::utils::timed_cache::TimedCache;
use massbit_data::{constraint_violation, store};
use std::{collections::BTreeMap, collections::HashMap, sync::Arc};
use std::{fmt, io::Write};
use std::{iter::FromIterator, time::Duration};
//use store::StoredDynamicDataSource;

/// The name of a database shard; valid names must match `[a-z0-9_]+`
#[derive(Clone, Debug, Eq, PartialEq, Hash, AsExpression, FromSqlRow)]
pub struct Shard(String);

lazy_static! {
    /// The name of the primary shard that contains all instance-wide data
    pub static ref PRIMARY_SHARD: Shard = Shard("primary".to_string());
}

/// How long to cache information about a deployment site
const SITES_CACHE_TTL: Duration = Duration::from_secs(120);

impl Shard {
    pub fn new(name: String) -> Result<Self, StoreError> {
        if name.is_empty() {
            return Err(StoreError::InvalidIdentifier(format!(
                "shard names must not be empty"
            )));
        }
        if name.len() > 30 {
            return Err(StoreError::InvalidIdentifier(format!(
                "shard names can be at most 30 characters, but `{}` has {} characters",
                name,
                name.len()
            )));
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(StoreError::InvalidIdentifier(format!(
                "shard names must only contain lowercase alphanumeric characters or '_'"
            )));
        }
        Ok(Shard(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Shard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromSql<Text, Pg> for Shard {
    fn from_sql(bytes: Option<&[u8]>) -> diesel::deserialize::Result<Self> {
        let s = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        Shard::new(s).map_err(Into::into)
    }
}

impl ToSql<Text, Pg> for Shard {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> diesel::serialize::Result {
        <String as ToSql<Text, Pg>>::to_sql(&self.0, out)
    }
}

/// Decide where a new deployment should be placed based on the subgraph name
/// and the network it is indexing. If the deployment can be placed, returns
/// the name of the database shard for the deployment and the names of the
/// indexers that should index it. The deployment should then be assigned to
/// one of the returned indexers.
pub trait DeploymentPlacer {
    fn place(&self, name: &str, network: &str) -> Result<Option<(Shard, Vec<NodeId>)>, String>;
}

/// Tools for managing unused deployments
pub mod unused {
    pub enum Filter {
        /// List all unused deployments
        All,
        /// List only deployments that are unused but have not been removed yet
        New,
    }
}

#[derive(Clone)]
pub struct IndexerStore {
    inner: Arc<IndexerStoreInner>,
}

impl IndexerStore {
    pub fn new(
        logger: &Logger,
        stores: Vec<(Shard, ConnectionPool, Vec<ConnectionPool>, Vec<usize>)>,
        placer: Arc<dyn DeploymentPlacer + Send + Sync + 'static>,
    ) -> Self {
        Self {
            inner: Arc::new(IndexerStoreInner::new(logger, stores, placer)),
        }
    }
}

impl std::ops::Deref for IndexerStore {
    type Target = IndexerStoreInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct IndexerStoreInner {
    logger: Logger,
    primary: ConnectionPool,
    stores: HashMap<Shard, Arc<DeploymentStore>>,
    /// Cache for the mapping from deployment id to shard/namespace/id. Only
    /// active sites are cached here to ensure we have a unique mapping from
    /// `SubgraphDeploymentId` to `Site`. The cache keeps entry only for
    /// `SITES_CACHE_TTL` so that changes, in particular, activation of a
    /// different deployment for the same hash propagate across different
    /// graph-node processes over time.
    sites: TimedCache<DeploymentHash, Site>,
    placer: Arc<dyn DeploymentPlacer + Send + Sync + 'static>,
}

impl IndexerStoreInner {
    pub fn new(
        logger: &Logger,
        stores: Vec<(Shard, ConnectionPool, Vec<ConnectionPool>, Vec<usize>)>,
        placer: Arc<dyn DeploymentPlacer + Send + Sync + 'static>,
    ) -> Self {
        let primary = stores
            .iter()
            .find(|(name, _, _, _)| name == &*PRIMARY_SHARD)
            .map(|(_, pool, _, _)| pool.clone())
            .expect("we always have a primary shard");
        let stores = HashMap::from_iter(stores.into_iter().map(
            |(name, main_pool, read_only_pools, weights)| {
                let logger = logger.new(o!("shard" => name.to_string()));

                (
                    name,
                    Arc::new(DeploymentStore::new(
                        &logger,
                        main_pool,
                        read_only_pools,
                        weights,
                    )),
                )
            },
        ));
        let sites = TimedCache::new(SITES_CACHE_TTL);
        IndexerStoreInner {
            logger: logger.clone(),
            primary,
            stores,
            sites,
            placer,
        }
    }
    fn cache_active(&self, site: &Arc<Site>) {
        if site.active {
            self.sites.set(site.deployment.clone(), site.clone());
        }
    }
    /// Return the active `Site` for this deployment hash
    fn site(&self, hash: &DeploymentHash) -> Result<Arc<Site>, StoreError> {
        if let Some(site) = self.sites.get(hash) {
            return Ok(site);
        }

        let conn = self.primary_conn()?;
        // let site = conn
        //     .find_active_site(hash)?
        //     .ok_or_else(|| StoreError::DeploymentNotFound(hash.to_string()))?;
        let site = Site::new(
            hash.cheap_clone(),
            PRIMARY_SHARD.clone(),
            String::from("sgd0"),
        );
        let site = Arc::new(site);

        self.cache_active(&site);
        Ok(site)
    }

    /// Return the store and site for the active deployment of this
    /// deployment hash
    fn store(
        &self,
        hash: &DeploymentHash,
    ) -> Result<(&Arc<DeploymentStore>, Arc<Site>), StoreError> {
        let site = self.site(hash)?;
        let store = self
            .stores
            .get(&site.shard)
            .ok_or(StoreError::UnknownShard(site.shard.as_str().to_string()))?;
        Ok((store, site))
    }
    /// Get a connection to the primary shard. Code must never hold one of these
    /// connections while also accessing a `DeploymentStore`, since both
    /// might draw connections from the same pool, and trying to get two
    /// connections can deadlock the entire process if the pool runs out
    /// of connections in between getting the first one and trying to get the
    /// second one.
    fn primary_conn(&self) -> Result<primary::Connection, StoreError> {
        let conn = self.primary.get_with_timeout_warning(&self.logger)?;
        Ok(primary::Connection::new(conn))
    }
    pub(crate) fn replica_for_query(
        &self,
        hash: DeploymentHash,
        for_subscription: bool,
    ) -> Result<(Arc<DeploymentStore>, Arc<Site>, ReplicaId), StoreError> {
        let (store, site) = self.store(&hash)?;
        let replica = store.replica_for_query(for_subscription)?;

        Ok((store.clone(), site.clone(), replica))
    }
}
//
// #[async_trait::async_trait]
// impl SubgraphStoreTrait for IndexerStore {
//     fn find_ens_name(&self, hash: &str) -> Result<Option<String>, QueryExecutionError> {
//         Ok(self.primary_conn()?.find_ens_name(hash)?)
//     }
//
//     // FIXME: This method should not get a node_id
//     fn create_subgraph_deployment(
//         &self,
//         name: SubgraphName,
//         schema: &Schema,
//         deployment: SubgraphDeploymentEntity,
//         node_id: NodeId,
//         network_name: String,
//         mode: SubgraphVersionSwitchingMode,
//     ) -> Result<DeploymentLocator, StoreError> {
//         self.create_deployment_internal(
//             name,
//             schema,
//             deployment,
//             node_id,
//             network_name,
//             mode,
//             false,
//         )
//     }
//
//     fn create_subgraph(&self, name: SubgraphName) -> Result<String, StoreError> {
//         let pconn = self.primary_conn()?;
//         pconn.transaction(|| pconn.create_subgraph(&name))
//     }
//
//     fn remove_subgraph(&self, name: SubgraphName) -> Result<(), StoreError> {
//         let pconn = self.primary_conn()?;
//         pconn.transaction(|| -> Result<_, StoreError> {
//             let changes = pconn.remove_subgraph(name)?;
//             pconn.send_store_event(&StoreEvent::new(changes))
//         })
//     }
//
//     fn reassign_subgraph(
//         &self,
//         deployment: &DeploymentLocator,
//         node_id: &NodeId,
//     ) -> Result<(), StoreError> {
//         let site = self.find_site(deployment.id.into())?;
//         let pconn = self.primary_conn()?;
//         pconn.transaction(|| -> Result<_, StoreError> {
//             let changes = pconn.reassign_subgraph(site.as_ref(), node_id)?;
//             pconn.send_store_event(&StoreEvent::new(changes))
//         })
//     }
//
//     fn assigned_node(&self, deployment: &DeploymentLocator) -> Result<Option<NodeId>, StoreError> {
//         let site = self.find_site(deployment.id.into())?;
//         let primary = self.primary_conn()?;
//         primary.assigned_node(site.as_ref())
//     }
//
//     fn assignments(&self, node: &NodeId) -> Result<Vec<DeploymentLocator>, StoreError> {
//         let primary = self.primary_conn()?;
//         primary
//             .assignments(node)
//             .map(|sites| sites.iter().map(|site| site.into()).collect())
//     }
//
//     fn subgraph_exists(&self, name: &SubgraphName) -> Result<bool, StoreError> {
//         let primary = self.primary_conn()?;
//         primary.subgraph_exists(name)
//     }
//
//     fn input_schema(&self, id: &DeploymentHash) -> Result<Arc<Schema>, StoreError> {
//         let (store, site) = self.store(&id)?;
//         let info = store.subgraph_info(site.as_ref())?;
//         Ok(info.input)
//     }
//
//     fn api_schema(&self, id: &DeploymentHash) -> Result<Arc<ApiSchema>, StoreError> {
//         let (store, site) = self.store(&id)?;
//         let info = store.subgraph_info(&site)?;
//         Ok(info.api)
//     }
//
//     fn writable(
//         &self,
//         deployment: &DeploymentLocator,
//     ) -> Result<Arc<dyn store::WritableStore>, StoreError> {
//         let site = self.find_site(deployment.id.into())?;
//         Ok(Arc::new(WritableStore::new(self.clone(), site)?))
//     }
//
//     fn writable_for_network_indexer(
//         &self,
//         id: &DeploymentHash,
//     ) -> Result<Arc<dyn WritableStoreTrait>, StoreError> {
//         let site = self.site(id)?;
//         Ok(Arc::new(WritableStore::new(self.clone(), site)?))
//     }
//
//     fn is_deployed(&self, id: &DeploymentHash) -> Result<bool, Error> {
//         match self.site(id) {
//             Ok(_) => Ok(true),
//             Err(StoreError::DeploymentNotFound(_)) => Ok(false),
//             Err(e) => Err(e.into()),
//         }
//     }
//
//     fn least_block_ptr(&self, id: &DeploymentHash) -> Result<Option<BlockPtr>, Error> {
//         let (store, site) = self.store(id)?;
//         store.block_ptr(site.as_ref())
//     }
//
//     /// Find the deployment locators for the subgraph with the given hash
//     fn locators(&self, hash: &str) -> Result<Vec<DeploymentLocator>, StoreError> {
//         Ok(self
//             .primary_conn()?
//             .find_sites(vec![hash.to_string()], false)?
//             .iter()
//             .map(|site| site.into())
//             .collect())
//     }
// }

/// A wrapper around `SubgraphStore` that only exposes functions that are
/// safe to call from `WritableStore`, i.e., functions that either do not
/// deal with anything that depends on a specific deployment
/// location/instance, or where the result is independent of the deployment
/// instance
struct WritableSubgraphStore(IndexerStore);
//
// impl WritableSubgraphStore {
//     fn primary_conn(&self) -> Result<primary::Connection, StoreError> {
//         self.0.primary_conn()
//     }
//
//     pub(crate) fn send_store_event(&self, event: &StoreEvent) -> Result<(), StoreError> {
//         self.0.send_store_event(event)
//     }
//
//     fn layout(&self, id: &DeploymentHash) -> Result<Arc<Layout>, StoreError> {
//         self.0.layout(id)
//     }
// }

// struct WritableStore {
//     store: WritableSubgraphStore,
//     writable: Arc<DeploymentStore>,
//     site: Arc<Site>,
// }
//
// impl WritableStore {
//     fn new(subgraph_store: IndexerStore, site: Arc<Site>) -> Result<Self, StoreError> {
//         let store = WritableSubgraphStore(subgraph_store.clone());
//         let writable = subgraph_store.for_site(site.as_ref())?.clone();
//         Ok(Self {
//             store,
//             writable,
//             site,
//         })
//     }
// }

// #[async_trait::async_trait]
// impl WritableStoreTrait for WritableStore {
//     fn block_ptr(&self) -> Result<Option<BlockPtr>, Error> {
//         self.writable.block_ptr(self.site.as_ref())
//     }
//
//     fn start_subgraph_deployment(&self, logger: &Logger) -> Result<(), StoreError> {
//         let store = &self.writable;
//
//         let graft_base = match store.graft_pending(&self.site.deployment)? {
//             Some((base_id, base_ptr)) => {
//                 let src = self.store.layout(&base_id)?;
//                 Some((src, base_ptr))
//             }
//             None => None,
//         };
//         store.start_subgraph(logger, self.site.clone(), graft_base)?;
//         self.store.primary_conn()?.copy_finished(self.site.as_ref())
//     }
//
//     fn revert_block_operations(&self, block_ptr_to: BlockPtr) -> Result<(), StoreError> {
//         let event = self
//             .writable
//             .revert_block_operations(self.site.clone(), block_ptr_to)?;
//         self.store.send_store_event(&event)
//     }
//
//     fn unfail(&self) -> Result<(), StoreError> {
//         self.writable.unfail(self.site.clone())
//     }
//
//     async fn fail_subgraph(&self, error: SubgraphError) -> Result<(), StoreError> {
//         self.writable
//             .fail_subgraph(self.site.deployment.clone(), error)
//             .await
//     }
//
//     fn supports_proof_of_indexing<'a>(self: Arc<Self>) -> DynTryFuture<'a, bool> {
//         self.writable
//             .clone()
//             .supports_proof_of_indexing(self.site.clone())
//     }
//
//     fn get(&self, key: EntityKey) -> Result<Option<Entity>, QueryExecutionError> {
//         self.writable.get(self.site.clone(), key)
//     }
//
//     fn transact_block_operations(
//         &self,
//         block_ptr_to: BlockPtr,
//         mods: Vec<EntityModification>,
//         stopwatch: StopwatchMetrics,
//         data_sources: Vec<StoredDynamicDataSource>,
//         deterministic_errors: Vec<SubgraphError>,
//     ) -> Result<(), StoreError> {
//         assert!(
//             same_subgraph(&mods, &self.site.deployment),
//             "can only transact operations within one shard"
//         );
//         let event = self.writable.transact_block_operations(
//             self.site.clone(),
//             block_ptr_to,
//             mods,
//             stopwatch.cheap_clone(),
//             data_sources,
//             deterministic_errors,
//         )?;
//
//         let _section = stopwatch.start_section("send_store_event");
//         self.store.send_store_event(&event)
//     }
//
//     fn get_many(
//         &self,
//         ids_for_type: BTreeMap<&EntityType, Vec<&str>>,
//     ) -> Result<BTreeMap<EntityType, Vec<Entity>>, StoreError> {
//         self.writable.get_many(self.site.clone(), ids_for_type)
//     }
//
//     async fn is_deployment_synced(&self) -> Result<bool, Error> {
//         Ok(self
//             .writable
//             .exists_and_synced(self.site.deployment.cheap_clone())
//             .await?)
//     }
//
//     fn unassign_subgraph(&self) -> Result<(), StoreError> {
//         let pconn = self.store.primary_conn()?;
//         pconn.transaction(|| -> Result<_, StoreError> {
//             let changes = pconn.unassign_subgraph(self.site.as_ref())?;
//             pconn.send_store_event(&StoreEvent::new(changes))
//         })
//     }
//
//     async fn load_dynamic_data_sources(&self) -> Result<Vec<StoredDynamicDataSource>, StoreError> {
//         self.writable
//             .load_dynamic_data_sources(self.site.deployment.clone())
//             .await
//     }
//
//     fn deployment_synced(&self) -> Result<(), Error> {
//         let event = {
//             // Make sure we drop `pconn` before we call into the deployment
//             // store so that we do not hold two database connections which
//             // might come from the same pool and could therefore deadlock
//             let pconn = self.store.primary_conn()?;
//             pconn.transaction(|| -> Result<_, Error> {
//                 let changes = pconn.promote_deployment(&self.site.deployment)?;
//                 Ok(StoreEvent::new(changes))
//             })?
//         };
//
//         self.writable.deployment_synced(&self.site.deployment)?;
//
//         Ok(self.store.send_store_event(&event)?)
//     }
// }
//
// fn same_subgraph(mods: &Vec<EntityModification>, id: &DeploymentHash) -> bool {
//     mods.iter().all(|md| &md.entity_key().subgraph_id == id)
// }
