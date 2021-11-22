use super::data::Entity;
use std::error::Error;

pub trait Store: Sync + Send {
    fn save(&mut self, entity_name: String, data: Entity);
    fn get(&mut self, entity_name: String, entity_id: &String) -> Option<Entity>;
    // fn query(
    //     &self,
    //     entity_type: String,
    //     filter: Option<EntityFilter>,
    //     order: EntityOrder,
    //     range: EntityRange,
    // ) -> Vec<Entity>;
    fn flush(&mut self, block_hash: &String, block_number: u64) -> Result<(), Box<dyn Error>>;
}
