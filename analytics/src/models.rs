use crate::postgres_queries::UpsertConflictFragment;
use crate::relational::{Column, Table};
use crate::schema::*;
use graph::prelude::Entity;

#[derive(Debug, Clone, Insertable, Queryable)]
#[table_name = "network_state"]
pub struct NetworkState {
    pub id: i64,
    pub chain: String,
    pub network: String,
    pub got_block: i64,
}

pub struct CommandData<'a> {
    pub table: &'a Table<'a>,
    pub columns: &'a Vec<Column>,
    pub values: &'a Vec<Entity>,
    pub conflict_fragment: &'a Option<UpsertConflictFragment<'a>>,
}
impl<'a> CommandData<'a> {
    pub fn new(
        table: &'a Table<'a>,
        columns: &'a Vec<Column>,
        values: &'a Vec<Entity>,
        conflict_fragment: &'a Option<UpsertConflictFragment<'a>>,
    ) -> Self {
        CommandData {
            table,
            columns,
            values,
            conflict_fragment,
        }
    }
}
