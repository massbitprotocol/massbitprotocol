use structmap::GenericMap;

pub trait Store: Sync + Send {
    fn save(&self, entity_name: String, data: GenericMap);
}

