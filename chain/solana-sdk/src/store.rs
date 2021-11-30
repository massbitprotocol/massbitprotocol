use crate::entity::Entity;
use std::error::Error;

pub trait IndexStore: Sync + Send {
    fn save(&mut self, entity_name: String, data: Entity);
    fn get(&mut self, entity_name: String, entity_id: &String) -> Option<Entity>;
    // fn query(
    //     &self,
    //     entity_type: String,
    //     filter: Option<EntityFilter>,
    //     order: EntityOrder,
    //     range: EntityRange,
    // ) -> Vec<Entity>;
    fn flush(&mut self, block_hash: &String, block_slot: u64) -> Result<(), Box<dyn Error>>;
}
