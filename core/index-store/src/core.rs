use diesel::{Insertable, Table, PgConnection, RunQueryDsl, Connection};
use diesel::query_builder::InsertStatement;
use diesel::query_dsl::load_dsl::ExecuteDsl;

pub struct IndexStore {
    pub connection_string: String,
}

// Refactor and use the logic from Mockstore here so we are not dependant on diesel https://github.com/massbitprotocol/massbitprotocol/pull/34/files#diff-cb28164240a17b00fd8ae0f15957d3159cad203dae86739b642a471c2d1931a0
impl IndexStore {
    pub fn save<T, M>(&self, table: T, records: M)
        where
            T: Table,
            M: Insertable<T>,
            InsertStatement<T, M::Values>: ExecuteDsl<PgConnection>,
    {
        println!("[Index Store] Writing to database");
        let connection = PgConnection::establish(&self.connection_string).expect(&format!("Error connecting to {}", self.connection_string));

        let _ = diesel::insert_into(table)
            .values(records)
            .execute(&connection);
    }
}