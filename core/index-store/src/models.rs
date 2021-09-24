use crate::schema::*;

#[derive(Debug, Clone, Insertable, Queryable)]
#[table_name = "indexers"]
pub struct Indexer {
    pub id: String,
    pub network: String,
    pub name: String,
    pub namespace: String,
    pub description: String,
    pub repo: String,
    pub manifest: String,
    pub index_status: String,
    pub got_block: i64,
    pub hash: String,
    pub v_id: i32
}
