use super::orm::schema::indexers;
#[derive(Insertable, Clone, Debug, Default)]
#[table_name = "indexers"]
pub struct IndexerEntity {
    pub network: Option<String>,
    pub name: String,
    pub namespace: String,
    pub description: Option<String>,
    pub repo: Option<String>,
    pub manifest: String,
    pub mapping: String,
    pub graphql: String,
    pub status: Option<String>,
    pub address: Option<String>,
    pub start_block: i64,
    pub got_block: i64,
    pub hash: String,
}
