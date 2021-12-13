pub mod models;
pub mod models_impl;
pub mod schema;

use diesel::insert_into;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};
// define your enum
#[derive(DbEnum, Clone, Debug, Serialize, Deserialize)]
pub enum IndexerStatus {
    Draft, // All variants must be fieldless
    Deploying,
    Deployed,
    Stopped,
    Invalid,
}

impl Default for IndexerStatus {
    fn default() -> IndexerStatus {
        IndexerStatus::Draft
    }
}
