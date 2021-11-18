use serde::Deserialize;
// The query parameters for indexer list.
#[derive(Debug, Deserialize)]
pub struct ListOptions {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}
