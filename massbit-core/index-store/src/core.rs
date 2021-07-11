use diesel::{Insertable, Table, PgConnection, RunQueryDsl, Connection};
use diesel::query_builder::InsertStatement;
use diesel::query_dsl::load_dsl::ExecuteDsl;

pub struct IndexStore {
    pub connection_string: String,
}

impl IndexStore {
    // Move MockStore of Đạt here
}