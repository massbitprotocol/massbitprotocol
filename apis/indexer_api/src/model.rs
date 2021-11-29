use serde::{Deserialize, Serialize};
// The query parameters for indexer list.
#[derive(Debug, Deserialize)]
pub struct ListOptions {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IndexerData {
    pub name: Option<String>,
    pub network: Option<String>,
    pub image_url: Option<String>,
    pub repository: Option<String>,
    pub description: Option<String>,
}
