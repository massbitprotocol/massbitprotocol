use crate::models::CommandData;
use crate::postgres_queries::UpsertConflictFragment;
use crate::relational::Table;
use massbit::prelude::{Entity, Value};
use std::collections::HashMap;

pub trait StorageAdapter: Sync + Send {
    //fn get_connection(&self) -> Result<Connection, anyhow::Error>;
    fn insert(&self, _table_name: &str, _value: HashMap<&str, Value>) -> Result<(), anyhow::Error> {
        unimplemented!()
    }
    fn upsert(
        &self,
        _table: &Table,
        _values: &Vec<Entity>,
        _conflict_fragment: &Option<UpsertConflictFragment>,
    ) -> Result<(), anyhow::Error> {
        unimplemented!()
    }

    /// Insert or update on conflict into some tables in one transaction
    fn transact_upserts(&self, _commands: Vec<CommandData>) -> Result<(), anyhow::Error> {
        unimplemented!()
    }
}

// enum StorageAdapterType {
//     Postgres,
//     BigQuery, //unimplemented
// }
