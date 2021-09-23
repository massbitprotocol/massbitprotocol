use crate::core::{IndexStore, QueryableStore, Store};
use crate::DEPLOYMENT_HASH;
use graph::blockchain::BlockHash;
use graph::cheap_clone::CheapClone;
use graph::components::store::{
    EntityCache, EntityFilter, EntityKey, EntityOrder, EntityRange, EntityType,
    ModificationsAndCache, StoreError, WritableStore,
};
use graph::components::subgraph::Entity;
use graph::prelude::{BlockPtr, StopwatchMetrics};
use graph_mock::MockMetricsRegistry;
use massbit_common::prelude::{
    slog::{self, Logger}
};
use std::convert::From;
use std::error::Error;
use std::sync::Arc;
use tokio::time::Instant;

pub struct IndexerState {
    pub store: Arc<dyn IndexStore>,
    pub entity_cache: EntityCache,
    stopwatch: StopwatchMetrics,
}
impl IndexerState {
    pub fn new(store: Arc<dyn IndexStore>) -> Self {
        let registry = Arc::new(MockMetricsRegistry::new());
        let stopwatch = StopwatchMetrics::new(
            Logger::root(slog::Discard, slog::o!()),
            DEPLOYMENT_HASH.cheap_clone(),
            registry.clone(),
        );
        //let writable_store = store.clone() as WritableStore
        let entity_cache = IndexerState::create_entity_cache(&store);
        IndexerState {
            store,
            entity_cache,
            stopwatch,
        }
    }
    pub fn create_entity_cache(store: &Arc<dyn IndexStore>) -> EntityCache {
        let writable_store: Arc<dyn WritableStore> = store.clone().to_writable_store();
        EntityCache::new(writable_store)
    }
}
impl QueryableStore for IndexerState {
    fn query(
        &self,
        entity_type: String,
        filter: Option<EntityFilter>,
        order: EntityOrder,
        range: EntityRange,
    ) -> Vec<Entity> {
        self.store.query(entity_type, filter, order, range)
    }
}
impl Store for IndexerState {
    fn save(&mut self, entity_type: String, data: Entity) {
        if let Ok(entity_id) = data.id() {
            let key = EntityKey {
                subgraph_id: crate::DEPLOYMENT_HASH.cheap_clone(),
                entity_type: EntityType::new(entity_type),
                entity_id,
            };
            //let entity = generic_map_to_entity(data);
            //let entity_cache = self.entity_cache.clone().lock().unwrap();
            self.entity_cache.set(key.clone(), data);
        }
    }
    fn get(&mut self, entity_type: String, entity_id: &String) -> Option<Entity> {
        let key = EntityKey {
            subgraph_id: crate::DEPLOYMENT_HASH.cheap_clone(),
            entity_type: EntityType::new(entity_type.clone()),
            entity_id: entity_id.clone(),
        };
        let mut result = None;
        if let Ok(cached_entity) = self.entity_cache.get(&key) {
            if cached_entity.is_some() {
                //log::info!("Get entity from cache in {:?}",start.elapsed());
                result = cached_entity
            }
        }
        if result.is_none() {
            match self.store.get(&key) {
                Ok(val) => {
                    //log::info!("Get entity from db in {:?}",start.elapsed());
                    result = val;
                }
                Err(err) => {
                    log::error!("{:?}", &err);
                }
            }
        }
        result
    }

    fn flush(&mut self, block_hash: &String, block_number: u64) -> Result<(), Box<dyn Error>> {
        //let mut data = self.entity_cache.lock().unwrap();

        let entity_cache = std::mem::replace(
            &mut self.entity_cache,
            Self::create_entity_cache(&self.store),
        );
        if let Ok(ModificationsAndCache {
            modifications: mods,
            data_sources: _,
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
                    hash: BlockHash::from(block_hash.as_bytes().to_vec()),
                    number: block_number as i32,
                };
                match self.store.transact_block_operations(
                    block_ptr,
                    mods,
                    self.stopwatch.cheap_clone(),
                    Vec::default(),
                    vec![],
                ) {
                    Ok(_) => {
                        log::info!("Transact block operation with {} records successfully in {:?}", length, start.elapsed());
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

