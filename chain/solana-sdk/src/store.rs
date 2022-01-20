use massbit_data::store::Entity;
use std::collections::HashMap;
use std::error::Error;
use transport::Value;

pub trait IndexStore: Sync + Send {
    //Deprecated - Use save_values
    fn save(&mut self, entity_name: String, data: Entity);
    fn save_values(&mut self, entity_name: &String, values: &HashMap<String, Value>);
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
