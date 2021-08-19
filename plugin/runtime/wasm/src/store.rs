use crate::graph::components::store::{EntityKey, EntityType, StoreError, WritableStore};
use crate::graph::data::query::error::QueryExecutionError;
use crate::graph::data::store::Entity;
pub use index_store::core::IndexStore;
use std::collections::BTreeMap;
/*
#[derive(Clone)]
pub struct IndexStore {}
impl IndexStore {
    pub fn new() -> IndexStore {
        IndexStore {}
    }
}
 */
impl WritableStore for IndexStore {
    fn get(&self, key: EntityKey) -> Result<Option<Entity>, QueryExecutionError> {
        println!("Call store get with key: {:?}", key);
        //todo!()
        let mut entity = Entity::new();
        let id = key.entity_id.as_str();
        entity.set("id", key.entity_id.as_str());
        match id {
            "steve" => {
                entity.set("name", "Steve");
                Ok(Some(entity))
            }
            _ => Ok(None),
        }
    }

    fn get_many(
        &self,
        ids_for_type: BTreeMap<&EntityType, Vec<&str>>,
    ) -> Result<BTreeMap<EntityType, Vec<Entity>>, StoreError> {
        println!("{:?}", ids_for_type);
        //todo!()
        let result: BTreeMap<EntityType, Vec<Entity>> = BTreeMap::default();
        Ok(result)
    }
}
