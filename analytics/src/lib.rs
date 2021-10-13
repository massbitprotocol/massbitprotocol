#[macro_use]
extern crate diesel;
extern crate diesel_migrations;

use diesel::{prelude::*, Connection, PgConnection};
use dotenv::dotenv;
use std::env;
pub mod ethereum;
pub mod models;
pub mod schema;
pub mod solana;
//pub mod substrate;
pub mod postgres_adapter;
pub mod postgres_queries;
pub mod relational;
pub mod sql_value;
pub mod storage_adapter;

use crate::models::NetworkState;
#[allow(unused_imports)]
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status, Streaming,
};
use tower::timeout::Timeout;

use crate::postgres_adapter::{PostgresAdapter, PostgresAdapterBuilder};
use massbit::firehose::bstream::{
    stream_client::StreamClient, BlockResponse, BlocksRequest, ChainType,
};
use massbit_common::NetworkType;

pub const GET_STREAM_TIMEOUT_SEC: u64 = 60;
pub const GET_BLOCK_TIMEOUT_SEC: u64 = 600;
pub const DEFAULT_DATABASE_URL: &str = "postgres://graph-node:let-me-in@localhost/analytic";
pub const MAX_POOL_SIZE: u32 = 50;

pub fn create_postgres_storage() -> PostgresAdapter {
    let database_url = env::var("DATABASE_URL").unwrap_or(String::from(DEFAULT_DATABASE_URL));
    let adapter_builder = PostgresAdapterBuilder::new().url(&database_url);
    adapter_builder.build()
}
pub fn establish_connection() -> PgConnection {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").unwrap_or(String::from(DEFAULT_DATABASE_URL));
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub async fn try_create_stream(
    client: &mut StreamClient<Timeout<Channel>>,
    chain_type: ChainType,
    start_block: i64,
    network: &Option<NetworkType>,
) -> Option<Streaming<BlockResponse>> {
    log::info!("Create new stream from block {}", start_block);
    let filter = vec![];
    let get_blocks_request = BlocksRequest {
        start_block_number: Some(start_block as u64),
        chain_type: chain_type as i32,
        network: network.clone().unwrap_or_default(),
        filter,
    };
    match client
        .blocks(Request::new(get_blocks_request.clone()))
        .await
    {
        Ok(res) => {
            return Some(res.into_inner());
        }
        Err(err) => {
            log::info!("Create new stream with error {:?}", &err);
        }
    }
    return None;
}

pub fn get_block_number(
    conn: &PgConnection,
    chain_value: String,
    network_value: String,
) -> Option<NetworkState> {
    use crate::schema::network_state::dsl::*;
    let results = network_state
        .filter(chain.eq(chain_value))
        .filter(network.eq(network_value))
        .limit(1)
        .load::<NetworkState>(conn)
        .expect("Error loading network state");
    if results.len() == 0 {
        None
    } else {
        match results.get(0) {
            Some(val) => Some(val.clone()),
            None => None,
        }
    }
}

#[macro_export]
macro_rules! create_columns {
    ($($att:expr => $exp:expr),*) => {
        {
            let mut temp_vec = Vec::new();
            $(
                temp_vec.push(Column::new($att, $exp));
            )*
            temp_vec
        }
    };
}

#[macro_export]
macro_rules! create_entity {
    ($($att:expr => $exp:expr),*) => {
        {
            let mut map : HashMap<Attribute, Value> = HashMap::default();
            $(
            map.insert(Attribute::from($att), Value::from($exp));
            )*
            Entity::from(map)
        }
    };
}
