//use super::postgres_queries::{ClampRangeQuery, FindManyQuery, FindQuery, InsertQuery};
use crate::diesel::OptionalExtension;
use crate::store::entity_cache::ModificationsAndCache;
use crate::store::EntityCache;
use chain_solana::types::{BlockPtr, BlockSlot};
use diesel::{ExpressionMethods, QueryDsl};
use indexer_orm::{models::Indexer, schema::*};
use massbit_common::prelude::bigdecimal::BigDecimal;
use massbit_common::prelude::diesel::r2d2::{ConnectionManager, PooledConnection};
use massbit_common::prelude::diesel::{Connection, PgConnection, RunQueryDsl};
use massbit_common::prelude::tokio::time::Instant;
use massbit_common::prelude::{anyhow, async_trait::async_trait, r2d2, slog::Logger};
use massbit_data::indexer::DeploymentHash;
use massbit_data::prelude::{CloneableAnyhowError, QueryExecutionError, StoreError};
use massbit_data::store::chain::BLOCK_NUMBER_MAX;
use massbit_data::store::{Entity, EntityKey, EntityModification, EntityType};
use massbit_solana_sdk::store::IndexStore;
use massbit_solana_sdk::transport::Value;
use massbit_storage_postgres::{
    relational::{Layout, DELETE_OPERATION_CHUNK_SIZE, POSTGRES_MAX_PARAMETERS},
    relational_queries::{ClampRangeQuery, EntityData, FindManyQuery, FindQuery, InsertQuery},
};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::ops::Deref;
use std::sync::Arc;

#[async_trait]
pub trait IndexerStoreTrait: Sync + Send {
    /// Looks up an entity using the given store key at the latest block.
    fn get(&self, key: &EntityKey) -> Result<Option<Entity>, QueryExecutionError>;

    /// Look up multiple entities as of the latest block. Returns a map of
    /// entities by type.
    fn get_many(
        &self,
        ids_for_type: BTreeMap<&EntityType, Vec<&str>>,
    ) -> Result<BTreeMap<EntityType, Vec<Entity>>, StoreError>;

    /// Transact the entity changes from a single block atomically into the store, and update the
    /// indexer block pointer to `block_ptr_to`.
    ///
    /// `block_ptr_to` must point to a child block of the current indexer block pointer.
    fn transact_block_operations(
        &self,
        block_ptr_to: BlockPtr,
        mods: Vec<EntityModification>,
    ) -> Result<(), StoreError>;
}
#[derive(Clone)]
pub struct IndexerStore {
    pub indexer_hash: String,
    pub connection_pool: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
    pub logger: Logger,
    pub layout: Layout,
}

impl IndexerStoreTrait for IndexerStore {
    fn get(&self, key: &EntityKey) -> Result<Option<Entity>, QueryExecutionError> {
        let conn = self.get_conn()?;
        //let layout = self.layout(&conn, site)?;

        // We should really have callers pass in a block number; but until
        // that is fully plumbed in, we just use the biggest possible block
        // number so that we will always return the latest version,
        // i.e., the one with an infinite upper bound
        let entity_type = key.entity_type.clone();
        let table = self.layout.table_for_entity(&entity_type)?;
        FindQuery::new(table.as_ref(), &key.entity_id, BLOCK_NUMBER_MAX)
            .get_result::<EntityData>(&conn)
            .optional()
            .map_err(|err| {
                let e: anyhow::Error = err.into();
                QueryExecutionError::StoreError(CloneableAnyhowError::from(e))
            })?
            .map(|entity_data| entity_data.deserialize_with_layout(&self.layout))
            .transpose()
            .map_err(|e| {
                println!("Error while get entity {:?}", &e);
                QueryExecutionError::EntityParseError(format!("Invalid entity {:?}", e))
            })
    }

    fn get_many(
        &self,
        ids_for_type: BTreeMap<&EntityType, Vec<&str>>,
    ) -> Result<BTreeMap<EntityType, Vec<Entity>>, StoreError> {
        if ids_for_type.is_empty() {
            return Ok(BTreeMap::new());
        }
        let conn = self
            .get_conn()
            .map_err(|err| StoreError::QueryExecutionError(format!("{:?}", &err)))?;
        self.layout.find_many(&conn, ids_for_type, BLOCK_NUMBER_MAX)
        // let mut tables = Vec::new();
        // for entity_name in ids_for_type.keys() {
        //     let entity_type = EntityType::new((*entity_name).clone());
        //     tables.push(self.layout.table_for_entity(&entity_type)?.as_ref());
        // }
        // let query = FindManyQuery {
        //     ids_for_type,
        //     tables,
        //     block: BLOCK_NUMBER_MAX,
        // };
        // let mut entities_for_type: BTreeMap<String, Vec<Entity>> = BTreeMap::new();
        // for data in query.load::<EntityData>(&conn)? {
        //     entities_for_type
        //         .entry(data.entity_type().into_string())
        //         .or_default()
        //         .push(data.deserialize_with_layout(&self.layout)?);
        // }
        // Ok(entities_for_type)
    }

    fn transact_block_operations(
        &self,
        block_ptr_to: BlockPtr,
        mods: Vec<EntityModification>,
    ) -> Result<(), StoreError> {
        let conn = self.get_conn()?;
        use indexer_deployments::dsl as d;
        use indexers::dsl as idx;
        conn.transaction(|| -> Result<_, StoreError> {
            // Emit a store event for the changes we are about to make. We
            // wait with sending it until we have done all our other work
            // so that we do not hold a lock on the notification queue
            // for longer than we have to
            //let event: StoreEvent = mods.iter().collect();
            //let section = stopwatch.start_section("apply_entity_modifications");
            let _count = self.apply_entity_modifications(&conn, mods, &block_ptr_to)?;
            //section.end();
            //Update context infos: synced block_hash, block_slot
            diesel::update(idx::indexers.filter(idx::hash.eq(&self.indexer_hash)))
                .set(idx::got_block.eq(block_ptr_to.number))
                .execute(&conn);
            diesel::update(d::indexer_deployments.filter(d::hash.eq(&self.indexer_hash)))
                .set((
                    d::latest_block_number.eq(BigDecimal::from(block_ptr_to.number)),
                    d::latest_block_hash.eq(block_ptr_to.hash.into_bytes()),
                ))
                .execute(&conn);
            Ok(())
        })
        //log::info!("{:?}", &event);
    }
}
impl IndexerStore {
    fn get_conn(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, QueryExecutionError> {
        self.connection_pool.get().map_err(|err| {
            let err: anyhow::Error = err.into();
            QueryExecutionError::StoreError(CloneableAnyhowError::from(err))
        })
    }
    fn apply_entity_modifications(
        &self,
        conn: &PgConnection,
        mods: Vec<EntityModification>,
        block_ptr: &BlockPtr,
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
            count += self.insert_entities(&entity_type, &mut entities, conn, block_ptr)? as i32;
        }
        // Overwrites:
        for (entity_type, mut entities) in overwrites.into_iter() {
            // we do not update the count since the number of entities remains the same
            self.overwrite_entities(&entity_type, &mut entities, conn, block_ptr)?;
        }

        // Removals
        for (entity_type, entity_keys) in removals.into_iter() {
            count -= self.remove_entities(&entity_type, &entity_keys, conn, block_ptr)? as i32;
        }
        Ok(count)
    }

    fn insert_entities(
        &self,
        entity_type: &EntityType,
        data: &mut [(EntityKey, Entity)],
        conn: &PgConnection,
        block_ptr: &BlockPtr,
    ) -> Result<usize, StoreError> {
        /*
        let section = stopwatch.start_section("check_interface_entity_uniqueness");
        for (key, _) in data.iter() {
            // WARNING: This will potentially execute 2 queries for each entity key.
            self.check_interface_entity_uniqueness(conn, key)?;
        }
        section.end();
         */
        //let _section = stopwatch.start_section("apply_entity_modifications_insert");
        let table = self.layout.table_for_entity(entity_type)?;
        let mut count = 0;
        // Each operation must respect the maximum number of bindings allowed in PostgreSQL queries,
        // so we need to act in chunks whose size is defined by the number of entities times the
        // number of attributes each entity type has.
        // We add 1 to account for the `block_range` bind parameter
        let chunk_size = POSTGRES_MAX_PARAMETERS / (table.columns.len() + 1);
        for chunk in data.chunks_mut(chunk_size) {
            count += InsertQuery::new(table, chunk, block_ptr.number)?
                .get_results(conn)
                .map(|ids| ids.len())?
        }
        Ok(count)
    }

    fn overwrite_entities(
        &self,
        entity_type: &EntityType,
        data: &mut [(EntityKey, Entity)],
        conn: &PgConnection,
        block_ptr: &BlockPtr,
    ) -> Result<usize, StoreError> {
        /*
        let section = stopwatch.start_section("check_interface_entity_uniqueness");
        for (key, _) in data.iter() {
            // WARNING: This will potentially execute 2 queries for each entity key.
            self.check_interface_entity_uniqueness(conn, layout, key)?;
        }
        section.end();
        */
        //let _section = stopwatch.start_section("apply_entity_modifications_update");
        let table = self.layout.table_for_entity(entity_type)?;
        let entity_keys: Vec<&str> = data.iter().map(|(key, _)| key.entity_id.as_str()).collect();

        //let section = stopwatch.start_section("update_modification_clamp_range_query");
        ClampRangeQuery::new(table, entity_type, &entity_keys, block_ptr.number).execute(conn)?;
        //section.end();

        //let _section = stopwatch.start_section("update_modification_insert_query");
        let mut count = 0;

        // Each operation must respect the maximum number of bindings allowed in PostgreSQL queries,
        // so we need to act in chunks whose size is defined by the number of entities times the
        // number of attributes each entity type has.
        // We add 1 to account for the `block_range` bind parameter
        let chunk_size = POSTGRES_MAX_PARAMETERS / (table.columns.len() + 1);
        for chunk in data.chunks_mut(chunk_size) {
            count += InsertQuery::new(table, chunk, block_ptr.number)?.execute(conn)?;
        }
        Ok(count)
    }

    fn remove_entities(
        &self,
        entity_type: &EntityType,
        entity_keys: &[String],
        conn: &PgConnection,
        block_ptr: &BlockPtr,
    ) -> Result<usize, StoreError> {
        //let _section = stopwatch.start_section("apply_entity_modifications_delete");
        // self.layout
        //     .delete(conn, entity_type, &entity_keys, block_ptr.number, stopwatch)
        //     .map_err(|_error| {
        //         anyhow::anyhow!("Failed to remove entities: {:?}", entity_keys).into()
        //     })
        let table = self.layout.table_for_entity(entity_type)?;
        let mut count = 0;
        for chunk in entity_keys.chunks(DELETE_OPERATION_CHUNK_SIZE) {
            count +=
                ClampRangeQuery::new(table, entity_type, chunk, block_ptr.number).execute(conn)?
        }
        Ok(count)
    }
}
pub struct CacheableStore {
    pub store: Arc<dyn IndexerStoreTrait>,
    pub entity_cache: EntityCache,
    pub indexer_id: String,
}

impl CacheableStore {
    pub fn new(store: Arc<dyn IndexerStoreTrait>, indexer_id: String) -> Self {
        let entity_cache = EntityCache::new(store.clone());
        CacheableStore {
            store,
            entity_cache,
            indexer_id,
        }
    }
}
impl IndexStore for CacheableStore {
    fn save(&mut self, entity_name: String, data: Entity) {
        if let Ok(entity_id) = data.id() {
            let key = EntityKey {
                indexer_hash: DeploymentHash::new(self.indexer_id.clone()).unwrap(),
                entity_type: EntityType::new(entity_name),
                entity_id,
            };
            self.entity_cache.set(key.clone(), data);
        }
    }

    fn save_values(&mut self, entity_name: &String, values: &HashMap<String, Value>) {
        todo!()
    }

    fn get(&mut self, entity_name: String, entity_id: &String) -> Option<Entity> {
        let key = EntityKey {
            indexer_hash: DeploymentHash::new(self.indexer_id.clone()).unwrap(),
            entity_type: EntityType::new(entity_name),
            entity_id: entity_id.clone(),
        };
        self.entity_cache.get(&key).unwrap_or({
            self.store.get(&key).unwrap_or(match self.store.get(&key) {
                Ok(val) => val,
                Err(err) => {
                    log::error!("{:?}", &err);
                    None
                }
            })
        })
    }

    fn flush(&mut self, block_hash: &String, block_slot: u64) -> Result<(), Box<dyn Error>> {
        //let mut data = self.entity_cache.lock().unwrap();
        let entity_cache =
            std::mem::replace(&mut self.entity_cache, EntityCache::new(self.store.clone()));
        if let Ok(ModificationsAndCache {
            modifications: mods,
            entity_lfu_cache: _cache,
        }) = entity_cache.as_modifications().map_err(|e| {
            log::error!("Error {:?}", e);
            StoreError::Unknown(e.into())
        }) {
            // Transact entity modifications into the store
            let length = mods.len();
            if length > 0 {
                let start = Instant::now();
                let block_ptr = BlockPtr {
                    hash: block_hash.clone(),
                    number: block_slot as BlockSlot,
                };
                match self.store.transact_block_operations(block_ptr, mods) {
                    Ok(_) => {
                        log::info!(
                            "Transact block operation with {} records successfully in {:?}",
                            length,
                            start.elapsed()
                        );
                    }
                    Err(err) => {
                        log::error!("Transact block operation with error {:?}", &err);
                    }
                }
            }
        }
        Ok(())
    }
}
