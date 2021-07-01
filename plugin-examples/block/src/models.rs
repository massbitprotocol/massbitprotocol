use super::schema::blocks;

#[derive(Insertable)]
#[table_name = "blocks"]
pub struct NewBlock {
    pub number: i64,
}
