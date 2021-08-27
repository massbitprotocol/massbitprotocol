pub mod store_builder;
use crate::prelude::{Arc, Logger};
use graph::components::metrics::stopwatch::StopwatchMetrics;
use graph::components::store::{
    EntityKey, EntityModification, EntityType, StoreError, StoreEvent, StoredDynamicDataSource,
    WritableStore,
};
use graph::components::subgraph::Entity;
use graph::data::query::QueryExecutionError;
use graph::data::subgraph::schema::SubgraphError;
use graph::ext::futures::{CancelHandle, CancelableError};
use graph::prelude::BlockPtr;
use graph::prelude::{BlockNumber, DynTryFuture};
use graph_store_postgres::command_support::Layout;
use graph_store_postgres::connection_pool::ConnectionPool;
use index_store::core::Store;
use massbit_common::prelude::diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    Connection, PgConnection,
};
use massbit_common::prelude::{
    anyhow::{anyhow, Error},
    async_trait::async_trait,
    log, structmap,
};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use store_builder::StoreBuilder;

pub const BLOCK_NUMBER_MAX: BlockNumber = <i32>::MAX;

#[derive(Clone)]
pub struct PostgresIndexStore {
    pub logger: Logger,
    pub connection: ConnectionPool,
    pub layout: Layout,
    //buffer: HashMap<String, TableBuffer>,
    //entity_dependencies: HashMap<String, Vec<String>>,
}

impl PostgresIndexStore {
    pub fn new(indexer: &str) -> Result<PostgresIndexStore, anyhow::Error> {
        let path = PathBuf::new();
        StoreBuilder::create_store(indexer, &path)
    }
}

impl Store for PostgresIndexStore {
    fn save(&mut self, entity_name: String, data: structmap::GenericMap) {}

    fn flush(&mut self) {}
}

#[async_trait]
impl WritableStore for PostgresIndexStore {
    fn block_ptr(&self) -> Result<Option<BlockPtr>, Error> {
        Ok(None)
    }

    fn start_subgraph_deployment(&self, logger: &Logger) -> Result<(), StoreError> {
        Ok(())
    }

    fn revert_block_operations(&self, block_ptr_to: BlockPtr) -> Result<(), StoreError> {
        Ok(())
    }

    fn unfail(&self) -> Result<(), StoreError> {
        Ok(())
    }

    async fn fail_subgraph(&self, error: SubgraphError) -> Result<(), StoreError> {
        Ok(())
    }

    fn supports_proof_of_indexing<'a>(self: Arc<Self>) -> DynTryFuture<'a, bool> {
        unimplemented!()
    }

    fn get(&self, key: EntityKey) -> Result<Option<Entity>, QueryExecutionError> {
        let conn = self.get_conn().map_err(|e| StoreError::Unknown(e))?;
        //let layout = self.layout(&conn, site)?;

        // We should really have callers pass in a block number; but until
        // that is fully plumbed in, we just use the biggest possible block
        // number so that we will always return the latest version,
        // i.e., the one with an infinite upper bound
        self.layout
            .find(&conn, &key.entity_type, &key.entity_id, BLOCK_NUMBER_MAX)
            .map_err(|e| {
                println!("Error while get entity {:?}", &e);
                QueryExecutionError::ResolveEntityError(
                    key.subgraph_id.clone(),
                    key.entity_type.to_string(),
                    key.entity_id.clone(),
                    format!("Invalid entity {:?}", e),
                )
            })
        /*
        let mut entity = Entity::new();
        let id = key.entity_id.as_str();
        entity.set("id", key.entity_id.as_str());
        println!("get entity by key {:?}", entity);
        match id {
            "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32" => {
                entity.set("pairCount", 0);
                Ok(Some(entity))
            }
            _ => Ok(None),
        }
         */
    }

    fn transact_block_operations(
        &self,
        block_ptr_to: BlockPtr,
        mods: Vec<EntityModification>,
        stopwatch: StopwatchMetrics,
        data_sources: Vec<StoredDynamicDataSource>,
        deterministic_errors: Vec<SubgraphError>,
    ) -> Result<(), StoreError> {
        /*
        mods.iter().for_each(|modification| {
            log::info!("Transact {:?}", modification);
        });
         */
        let conn = self.get_conn()?;
        let event = conn.transaction(|| -> Result<_, StoreError> {
            // Emit a store event for the changes we are about to make. We
            // wait with sending it until we have done all our other work
            // so that we do not hold a lock on the notification queue
            // for longer than we have to
            let event: StoreEvent = mods.iter().collect();

            let section = stopwatch.start_section("apply_entity_modifications");
            let count = self.apply_entity_modifications(&conn, mods, &block_ptr_to, stopwatch)?;
            section.end();
            /*
            deployment::update_entity_count(
                &conn,
                site.as_ref(),
                layout.count_query.as_str(),
                count,
            )?;
            section.end();

            dynds::insert(&conn, &site.deployment, data_sources, &block_ptr_to)?;

            if !deterministic_errors.is_empty() {
                deployment::insert_subgraph_errors(
                    &conn,
                    &site.deployment,
                    deterministic_errors,
                    block_ptr_to.block_number(),
                )?;
            }

            deployment::forward_block_ptr(&conn, &site.deployment, block_ptr_to)?;
            */
            Ok(event)
        })?;
        log::info!("{:?}", event);
        Ok(())
    }

    fn get_many(
        &self,
        ids_for_type: BTreeMap<&EntityType, Vec<&str>>,
    ) -> Result<BTreeMap<EntityType, Vec<Entity>>, StoreError> {
        log::info!("Get many ids for type {:?}", ids_for_type);
        if ids_for_type.is_empty() {
            return Ok(BTreeMap::new());
        }
        let conn = self.get_conn()?;
        self.layout.find_many(&conn, ids_for_type, BLOCK_NUMBER_MAX)
    }

    fn deployment_synced(&self) -> Result<(), Error> {
        todo!()
    }

    async fn is_deployment_synced(&self) -> Result<bool, Error> {
        todo!()
    }

    fn unassign_subgraph(&self) -> Result<(), StoreError> {
        todo!()
    }

    async fn load_dynamic_data_sources(&self) -> Result<Vec<StoredDynamicDataSource>, StoreError> {
        todo!()
    }
    /*
    fn transact_block_operations(&self, mods: Vec<EntityModification>) -> Result<(), StoreError> {
        mods.iter()
            .for_each(|modification| println!("{:?}", modification));
        self.with_conn(|conn, _| {
            let event = conn.transaction(|| -> Result<_, StoreError> {
                // Emit a store event for the changes we are about to make. We
                // wait with sending it until we have done all our other work
                // so that we do not hold a lock on the notification queue
                // for longer than we have to
                let event: StoreEvent = mods.iter().collect();

                //let section = stopwatch.start_section("apply_entity_modifications");
                let count = self.apply_entity_modifications(&conn, mods)?;
                /*
                deployment::update_entity_count(
                    &conn,
                    site.as_ref(),
                    layout.count_query.as_str(),
                    count,
                )?;
                //section.end();

                dynds::insert(&conn, &site.deployment, data_sources, &block_ptr_to)?;

                if !deterministic_errors.is_empty() {
                    deployment::insert_subgraph_errors(
                        &conn,
                        &site.deployment,
                        deterministic_errors,
                        block_ptr_to.block_number(),
                    )?;
                }

                deployment::forward_block_ptr(&conn, &site.deployment, block_ptr_to)?;
                */
                Ok(event)
            })?;
            event
        });
        Ok(())
    }
     */
}

impl PostgresIndexStore {
    /*
    /// Execute a closure with a connection to the database.
    ///
    /// # API
    ///   The API of using a closure to bound the usage of the connection serves several
    ///   purposes:
    ///
    ///   * Moves blocking database access out of the `Future::poll`. Within
    ///     `Future::poll` (which includes all `async` methods) it is illegal to
    ///     perform a blocking operation. This includes all accesses to the
    ///     database, acquiring of locks, etc. Calling a blocking operation can
    ///     cause problems with `Future` combinators (including but not limited
    ///     to select, timeout, and FuturesUnordered) and problems with
    ///     executors/runtimes. This method moves the database work onto another
    ///     thread in a way which does not block `Future::poll`.
    ///
    ///   * Limit the total number of connections. Because the supplied closure
    ///     takes a reference, we know the scope of the usage of all entity
    ///     connections and can limit their use in a non-blocking way.
    ///
    /// # Cancellation
    ///   The normal pattern for futures in Rust is drop to cancel. Once we
    ///   spawn the database work in a thread though, this expectation no longer
    ///   holds because the spawned task is the independent of this future. So,
    ///   this method provides a cancel token which indicates that the `Future`
    ///   has been dropped. This isn't *quite* as good as drop on cancel,
    ///   because a drop on cancel can do things like cancel http requests that
    ///   are in flight, but checking for cancel periodically is a significant
    ///   improvement.
    ///
    ///   The implementation of the supplied closure should check for cancel
    ///   between every operation that is potentially blocking. This includes
    ///   any method which may interact with the database. The check can be
    ///   conveniently written as `token.check_cancel()?;`. It is low overhead
    ///   to check for cancel, so when in doubt it is better to have too many
    ///   checks than too few.
    ///
    /// # Panics:
    ///   * This task will panic if the supplied closure panics
    ///   * This task will panic if the supplied closure returns Err(Cancelled)
    ///     when the supplied cancel token is not cancelled.
    pub(crate) async fn with_conn<T: Send + 'static>(
        &self,
        f: impl 'static
            + Send
            + FnOnce(
                &PooledConnection<ConnectionManager<PgConnection>>,
                &CancelHandle,
            ) -> Result<T, CancelableError<StoreError>>,
    ) -> Result<T, StoreError> {
        self.connection.with_conn(f).await
    }
     */
    fn get_conn(&self) -> Result<PooledConnection<ConnectionManager<PgConnection>>, Error> {
        self.connection.get_with_timeout_warning(&self.logger)
    }
    fn apply_entity_modifications(
        &self,
        conn: &PgConnection,
        mods: Vec<EntityModification>,
        block_ptr: &BlockPtr,
        stopwatch: StopwatchMetrics,
    ) -> Result<i32, StoreError> {
        use EntityModification::*;
        let mut count = 0;

        // Group `Insert`s and `Overwrite`s by key, and accumulate `Remove`s.
        let mut inserts = HashMap::new();
        let mut overwrites = HashMap::new();
        let mut removals = HashMap::new();
        for modification in mods.into_iter() {
            log::info!("Store {:?}", &modification);
            match modification {
                Insert { key, data } => {
                    inserts
                        .entry(key.entity_type.clone())
                        .or_insert_with(Vec::new)
                        .push((key, data));
                }
                Overwrite { key, data } => {
                    overwrites
                        .entry(key.entity_type.clone())
                        .or_insert_with(Vec::new)
                        .push((key, data));
                }
                Remove { key } => {
                    removals
                        .entry(key.entity_type.clone())
                        .or_insert_with(Vec::new)
                        .push(key.entity_id);
                }
            }
        }

        // Apply modification groups.
        // Inserts:
        for (entity_type, mut entities) in inserts.into_iter() {
            count +=
                self.insert_entities(&entity_type, &mut entities, conn, block_ptr, &stopwatch)?
                    as i32
        }

        // Overwrites:
        for (entity_type, mut entities) in overwrites.into_iter() {
            // we do not update the count since the number of entities remains the same
            self.overwrite_entities(&entity_type, &mut entities, conn, block_ptr, &stopwatch)?;
        }

        // Removals
        for (entity_type, entity_keys) in removals.into_iter() {
            count -=
                self.remove_entities(&entity_type, &entity_keys, conn, block_ptr, &stopwatch)?
                    as i32;
        }
        Ok(count)
    }

    fn insert_entities(
        &self,
        entity_type: &EntityType,
        data: &mut [(EntityKey, Entity)],
        conn: &PgConnection,
        block_ptr: &BlockPtr,
        stopwatch: &StopwatchMetrics,
    ) -> Result<usize, StoreError> {
        /*
        let section = stopwatch.start_section("check_interface_entity_uniqueness");
        for (key, _) in data.iter() {
            // WARNING: This will potentially execute 2 queries for each entity key.
            self.check_interface_entity_uniqueness(conn, layout, key)?;
        }
        section.end();
        */
        let _section = stopwatch.start_section("apply_entity_modifications_insert");
        self.layout
            .insert(conn, entity_type, data, block_ptr.number, stopwatch)
    }

    fn overwrite_entities(
        &self,
        entity_type: &EntityType,
        data: &mut [(EntityKey, Entity)],
        conn: &PgConnection,
        block_ptr: &BlockPtr,
        stopwatch: &StopwatchMetrics,
    ) -> Result<usize, StoreError> {
        /*
        let section = stopwatch.start_section("check_interface_entity_uniqueness");
        for (key, _) in data.iter() {
            // WARNING: This will potentially execute 2 queries for each entity key.
            self.check_interface_entity_uniqueness(conn, layout, key)?;
        }
        section.end();
        */
        let _section = stopwatch.start_section("apply_entity_modifications_update");
        log::info!("Update entity {:?} with value {:?}", &entity_type, data);
        self.layout
            .update(conn, &entity_type, data, block_ptr.number, stopwatch)
    }

    fn remove_entities(
        &self,
        entity_type: &EntityType,
        entity_keys: &[String],
        conn: &PgConnection,
        block_ptr: &BlockPtr,
        stopwatch: &StopwatchMetrics,
    ) -> Result<usize, StoreError> {
        let _section = stopwatch.start_section("apply_entity_modifications_delete");
        self.layout
            .delete(conn, entity_type, &entity_keys, block_ptr.number, stopwatch)
            .map_err(|_error| anyhow!("Failed to remove entities: {:?}", entity_keys).into())
    }
}
