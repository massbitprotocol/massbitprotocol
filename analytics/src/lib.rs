#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
pub mod models;
pub mod schema;
pub mod ethereum;
pub mod solana;
pub mod substrate;
pub mod stream_mod {
    tonic::include_proto!("chaindata");
}