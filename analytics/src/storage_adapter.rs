use std::collections::HashMap;

pub trait StorageAdapter {
    fn insert() -> Result<(), anyhow::Error> {
        Ok(())
    }
    fn upsert() -> Result<(), anyhow::Error> {
        Ok(())
    }
}

enum StorageAdapterType {
    Postgres,
}
