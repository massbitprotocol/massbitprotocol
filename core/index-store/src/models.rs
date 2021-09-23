use crate::schema::*;

#[derive(Debug, Clone, Insertable, Queryable)]
#[table_name = "indexer_state"]
pub struct IndexerState {
    pub id: i64,
    pub indexer_hash: String,
    pub schema_name: String,
    pub got_block: i64
}