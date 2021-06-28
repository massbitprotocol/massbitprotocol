use diesel::prelude::*;

#[derive(Table, Insertable, Queryable)]
#[table_name = "blocks"]
pub struct Block {
    #[column_type = "BigInt"]
    pub id: i64,
}

impl Block {
    pub fn save(self) {
        unsafe {
            let conn = super::CONN.as_ref().unwrap();
            let inserted_block = diesel::insert_into(blocks::table)
                .values(&self)
                .execute(conn);
        }
    }
}

#[derive(Table, Insertable, Queryable)]
#[table_name = "extrinsics"]
pub struct Extrinsic {
    #[column_type = "BigInt"]
    pub id: i64,
}

impl Extrinsic {
    pub fn save(self) {
        unsafe {
            let conn = super::CONN.as_ref().unwrap();
            let inserted_block = diesel::insert_into(extrinsics::table)
                .values(&self)
                .execute(conn);
        }
    }
}
