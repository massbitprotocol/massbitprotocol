pub mod relational;
pub mod store_builder;
use graph::components::metrics::stopwatch::StopwatchMetrics;
use graph::components::store::{
    EntityCollection, EntityFilter, EntityKey, EntityModification, EntityOrder, EntityRange,
    EntityType, StoreError, StoreEvent, StoredDynamicDataSource, WritableStore,
};
use graph::components::subgraph::Entity;
use graph::data::query::QueryExecutionError;
use graph::data::subgraph::schema::SubgraphError;
use graph::prelude::BlockPtr;
use graph::prelude::{BlockNumber, DynTryFuture};
use graph_store_postgres::command_support::Layout;
use graph_store_postgres::connection_pool::ConnectionPool;
use massbit_common::prelude::anyhow;
use massbit_common::prelude::diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    Connection, PgConnection,
};
use massbit_common::prelude::slog::Logger;
use std::sync::Arc;

use crate::core::{IndexStore, QueryableStore};
use crate::postgres::relational::LayoutExt;
use crate::Value;
use massbit_common::prelude::{
    anyhow::{anyhow, Error},
    async_trait::async_trait,
    log,
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
    //pub entity_dependencies: HashMap<EntityType, HashSet<EntityType>>,
}

impl PostgresIndexStore {
    pub fn new(indexer: &str) -> Result<PostgresIndexStore, anyhow::Error> {
        let path = PathBuf::new();
        StoreBuilder::create_store(indexer, &path)
    }
}

impl QueryableStore for PostgresIndexStore {
    fn query(
        &self,
        entity_type: String,
        filter: Option<EntityFilter>,
        order: EntityOrder,
        range: EntityRange,
    ) -> Vec<Entity> {
        match self.get_conn() {
            Ok(conn) => {
                match self.layout.filter::<Entity>(
                    &self.logger,
                    &conn,
                    EntityType::new(entity_type),
                    filter,
                    order,
                    range,
                ) {
                    Ok(vec) => vec,
                    Err(err) => {
                        log::error!("{:?}", &err);
                        vec![]
                    }
                }
            }
            Err(err) => {
                log::error!("{:?}", &err);
                vec![]
            }
        }
    }
}
impl IndexStore for PostgresIndexStore {}
#[async_trait]
impl WritableStore for PostgresIndexStore {
    fn block_ptr(&self) -> Result<Option<BlockPtr>, Error> {
        Ok(None)
    }

    fn start_subgraph_deployment(&self, _logger: &Logger) -> Result<(), StoreError> {
        Ok(())
    }

    fn revert_block_operations(&self, _block_ptr_to: BlockPtr) -> Result<(), StoreError> {
        Ok(())
    }

    fn unfail(&self) -> Result<(), StoreError> {
        Ok(())
    }

    async fn fail_subgraph(&self, _error: SubgraphError) -> Result<(), StoreError> {
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
    }

    fn transact_block_operations(
        &self,
        block_ptr_to: BlockPtr,
        mods: Vec<EntityModification>,
        stopwatch: StopwatchMetrics,
        _data_sources: Vec<StoredDynamicDataSource>,
        _deterministic_errors: Vec<SubgraphError>,
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
            let _count = self.apply_entity_modifications(&conn, mods, &block_ptr_to, stopwatch)?;
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
        log::info!("{:?}", &event);
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
}

impl PostgresIndexStore {
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
            //log::info!("Store modification {:?}", &modification);
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
        for (entity_type, mut entities) in inserts.into_iter() {
            count +=
                self.insert_entities(&entity_type, &mut entities, conn, block_ptr, &stopwatch)?
                    as i32;
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
            self.check_interface_entity_uniqueness(conn, key)?;
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
        //log::info!("Update entity {:?} with value {:?}", &entity_type, data);
        //Original code update current record and insert new one
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
