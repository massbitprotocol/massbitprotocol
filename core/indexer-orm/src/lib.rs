#[macro_use]
extern crate diesel;
extern crate diesel_derive_enum;
extern crate diesel_migrations;
pub mod models;
pub mod schema;

pub use models::IndexerHealthMapping;
pub use models::IndexerStatusMapping;
pub type DieselBlockSlot = diesel::sql_types::BigInt;
