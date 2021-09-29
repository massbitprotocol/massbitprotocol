use std::collections::HashMap;
use massbit_common::prelude::diesel::Connection;
use massbit_common::prelude::diesel::r2d2::{PooledConnection, ConnectionManager};
use massbit_common::prelude::r2d2;
use graph::prelude::{Value, Entity};
use crate::relational::{Column, Table};
use crate::postgres_queries::UpsertConflictFragment;

pub trait StorageAdapter : Sync + Send {
    //fn get_connection(&self) -> Result<Connection, anyhow::Error>;
    fn insert(&self, table_name: &str, value: HashMap<&str, Value>) -> Result<(), anyhow::Error> {
        unimplemented!()
    }
    fn upsert(&self, table: &Table, columns: &Vec<Column>, value: &Vec<Entity>, conflict_fragment: Option<UpsertConflictFragment>)
        -> Result<(), anyhow::Error> {
        unimplemented!()
    }
}

enum StorageAdapterType {
    Postgres,
    BigQuery        //unimplemented
}
