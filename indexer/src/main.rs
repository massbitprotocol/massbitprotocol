use ormx::{Insert, Table};
use sqlx::PgPool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = "abc".to_string();
    let db = PgPool::connect(&database_url).await?;
    let mut block = Block {
        id: 1,
    }.insert()
    Ok(())
}

#[derive(Debug, ormx::Table)]
#[ormx(table = "blocks", insertable)]
pub struct Block {
    pub id: i64,
}
