use crate::core::Store;
use crate::DEPLOYMENT_HASH;
use crate::PRIMARY_KEY_COLUMN;
use graph::blockchain::BlockHash;
use graph::cheap_clone::CheapClone;
use graph::components::store::{
    EntityCache, EntityKey, EntityType, ModificationsAndCache, StoreError, WritableStore,
};
use graph::components::subgraph::Entity;
use graph::data::store::Value as StoreValue;
use graph::data::subgraph::DeploymentHash;
use graph::prelude::{Attribute, BigDecimal, BigInt, BlockPtr, StopwatchMetrics};
use graph_mock::MockMetricsRegistry;
use inflector::cases::camelcase::to_camel_case;
use inflector::Inflector;
use massbit_common::prelude::structmap::value::{Num, Value};
use massbit_common::prelude::{
    slog::{self, Logger},
    structmap::GenericMap,
};
use std::collections::HashMap;
use std::convert::From;
use std::error::Error;
use std::sync::{Arc, Mutex};
/*
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(FromMap)]
#[derive(Default, Clone, ToMap)]
pub struct SolanaBlockTs {
    pub id: String,
    pub block_height: i64,
    pub block_hash: String,
    pub timestamp: String,
}
*/

pub struct IndexerState {
    pub store: Arc<dyn WritableStore>,
    pub entity_cache: EntityCache,
    stopwatch: StopwatchMetrics,
}
impl IndexerState {
    pub fn new(store: Arc<dyn WritableStore>) -> Self {
        let registry = Arc::new(MockMetricsRegistry::new());
        let stopwatch = StopwatchMetrics::new(
            Logger::root(slog::Discard, slog::o!()),
            DEPLOYMENT_HASH.cheap_clone(),
            registry.clone(),
        );
        let entity_cache = EntityCache::new(store.clone());
        IndexerState {
            store,
            entity_cache,
            stopwatch,
        }
    }
}
impl Store for IndexerState {
    fn save(&mut self, entity_type: String, data: GenericMap) {
        if let Some(entity_id) = data["id"].string() {
            let key = EntityKey {
                subgraph_id: crate::DEPLOYMENT_HASH.cheap_clone(),
                entity_type: EntityType::new(entity_type),
                entity_id,
            };
            let entity = generic_map_to_entity(data);
            //let entity_cache = self.entity_cache.clone().lock().unwrap();
            self.entity_cache.set(key.clone(), entity);
        }
    }
    fn flush(&mut self, block_hash: &String, block_number: u64) -> Result<(), Box<dyn Error>> {
        //let mut data = self.entity_cache.lock().unwrap();
        let entity_cache =
            std::mem::replace(&mut self.entity_cache, EntityCache::new(self.store.clone()));
        if let Ok(ModificationsAndCache {
            modifications: mods,
            data_sources,
            entity_lfu_cache: cache,
        }) = entity_cache.as_modifications().map_err(|e| {
            log::error!("Error {:?}", e);
            StoreError::Unknown(e.into())
        }) {
            // Transact entity modifications into the store
            if mods.len() > 0 {
                //let store = self.store.clone();
                let block_ptr = BlockPtr {
                    hash: Default::default(),
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

fn generic_map_to_entity(m: GenericMap) -> Entity {
    Entity::from(
        m.iter()
            .map(|(key, val)| (key.clone(), generic_value_to_store(val)))
            .collect::<HashMap<Attribute, StoreValue>>(),
    )
}
fn generic_value_to_store(value: &Value) -> StoreValue {
    match value {
        Value::Null => StoreValue::Null,
        Value::Bool(v) => StoreValue::Bool(*v),
        Value::Num(num) => match num {
            Num::I64(v) => StoreValue::BigInt(BigInt::from(*v)),
            Num::U64(v) => StoreValue::BigInt(BigInt::from(*v)),
            Num::F64(v) => StoreValue::BigDecimal(BigDecimal::from(*v)),
        },
        Value::String(v) => StoreValue::String(v.clone()),
        Value::Array(arr) => StoreValue::List(
            arr.iter()
                .map(|v| generic_value_to_store(v))
                .collect::<Vec<StoreValue>>(),
        ),
    }
}
