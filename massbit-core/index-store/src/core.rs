use diesel::{Insertable, Table, PgConnection, RunQueryDsl, Connection};
use diesel::query_builder::InsertStatement;
use diesel::query_dsl::load_dsl::ExecuteDsl;

pub struct IndexStore {
    pub connection_string: String,
}

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