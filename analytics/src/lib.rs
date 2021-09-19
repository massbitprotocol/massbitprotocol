#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use diesel::{PgConnection, Connection};
use std::env;
use dotenv::dotenv;
pub mod schema;
pub mod ethereum;
pub mod solana;
pub mod substrate;
pub mod stream_mod {
    tonic::include_proto!("chaindata");
}

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}