use structmap::GenericMap;

pub trait Store {
    fn save(&self, entity: GenericMap);
}
