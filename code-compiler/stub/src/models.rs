use super::schema::blocks;
use diesel::{PgConnection, Connection, RunQueryDsl};

#[derive(Insertable)]
#[table_name = "blocks"]
pub struct NewBlock {
    pub number: i64,
}