use structmap::GenericMap;

pub trait Store {
    fn save(&self, entity_name: String, data: GenericMap);
}
