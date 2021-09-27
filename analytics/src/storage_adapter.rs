use std::collections::HashMap;

pub trait StorageAdapter : Sync + Send {
    fn insert(&self) -> Result<(), anyhow::Error> {
        unimplemented!()
    }
    fn upsert(&self, table_name: String)
        -> Result<(), anyhow::Error> {
        unimplemented!()
    }
}

enum StorageAdapterType {
    Postgres,
}
