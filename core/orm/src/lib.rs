#[macro_use]
extern crate diesel;
extern crate diesel_migrations;
#[macro_use]
extern crate diesel_derive_enum;
pub mod models;
pub mod schema;

pub use models::IndexerHealthMapping;
