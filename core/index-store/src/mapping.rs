use crate::core::{IndexStore, QueryableStore, Store, ToWritableStore};
use crate::DEPLOYMENT_HASH;
use graph::blockchain::BlockHash;
use graph::cheap_clone::CheapClone;
use graph::components::store::{
    EntityCache, EntityFilter, EntityKey, EntityOrder, EntityRange, EntityType,
    ModificationsAndCache, StoreError, WritableStore,
};
use graph::components::subgraph::Entity;
use graph::data::query::QueryExecutionError;
use graph::data::store::Value as StoreValue;
use graph::prelude::{q, Attribute, BigDecimal, BigInt, BlockPtr, StopwatchMetrics};
use graph_mock::MockMetricsRegistry;
//use massbit_common::prelude::structmap::value::{Num, Value};
use massbit_common::prelude::{
    slog::{self, Logger},
    structmap::GenericMap,
};
use std::collections::{BTreeMap, HashMap};
use std::convert::From;
use std::error::Error;
use std::sync::Arc;

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
                result = cached_entity
            }
        }
        if result.is_none() {
            match self.store.get(&key) {
                Ok(val) => {
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
            if mods.len() > 0 {
                //let store = self.store.clone();
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
                        log::info!("Transact block operation successfully");
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

// fn generic_map_to_entity(m: GenericMap) -> Entity {
//     Entity::from(
//         m.iter()
//             .map(|(key, val)| (key.clone(), generic_value_to_store(val)))
//             .collect::<HashMap<Attribute, StoreValue>>(),
//     )
// }
// fn generic_value_to_store(value: &Value) -> StoreValue {
//     match value {
//         Value::Null => StoreValue::Null,
//         Value::Bool(v) => StoreValue::Bool(*v),
//         Value::Num(num) => match num {
//             Num::I64(v) => StoreValue::BigInt(BigInt::from(*v)),
//             Num::U64(v) => StoreValue::BigInt(BigInt::from(*v)),
//             Num::F64(v) => StoreValue::BigDecimal(BigDecimal::from(*v)),
//         },
//         Value::String(v) => StoreValue::String(v.clone()),
//         Value::Array(arr) => StoreValue::List(
//             arr.iter()
//                 .map(|v| generic_value_to_store(v))
//                 .collect::<Vec<StoreValue>>(),
//         ),
//     }
// }
//
// fn entity_to_generic_map(e: Entity) -> GenericMap {
//     let map: BTreeMap<String, q::Value> = BTreeMap::from(e);
//     map.iter()
//         .map(|(key, val)| (key.clone(), store_to_generic_value(val)))
//         .collect::<GenericMap>()
// }
//
// fn store_to_generic_value(value: &q::Value) -> Value {
//     match value {
//         q::Value::Variable(var) => Value::String(var.to_string()),
//         q::Value::Int(num) => Value::Num(Num::U64(num.as_i64().unwrap() as u64)),
//         q::Value::Float(f) => Value::Num(Num::F64(*f)),
//         q::Value::String(s) => Value::String(s.clone()),
//         q::Value::Boolean(b) => Value::Bool(*b),
//         q::Value::List(vec) => Value::Array(
//             vec.into_iter()
//                 .map(|elm| store_to_generic_value(elm))
//                 .collect::<Vec<Value>>(),
//         ),
//         q::Value::Null => Value::Null,
//         q::Value::Enum(e) => Value::String(e.clone()),
//         q::Value::Object(map) => Value::Null,
//     }
// }
