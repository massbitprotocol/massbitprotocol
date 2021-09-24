use crate::schema::*;

#[derive(Debug, Clone, Insertable, Queryable)]
#[table_name = "network_state"]
pub struct NetworkState {
    pub id: i64,
    pub chain: String,
    pub network: String,
    pub got_block: i64
}