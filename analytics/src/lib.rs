#[macro_use]
extern crate diesel;
extern crate diesel_migrations;

use std::env;
use diesel::{prelude::*, Connection, PgConnection};
use dotenv::dotenv;
pub mod manager;
pub mod schema;
pub mod models;
pub mod ethereum;
pub mod solana;
//pub mod substrate;
pub mod storage_adapter;
pub mod postgres_adapter;
pub mod postgres_queries;
pub mod relational;
pub mod sql_value;
pub mod util;
pub mod stream_mod {
    tonic::include_proto!("chaindata");
}
use crate::models::NetworkState;
use tower::timeout::Timeout;
#[allow(unused_imports)]
use tonic::{
    Request,
    Response, Status, transport::{Channel, Server},
    Streaming
};

use crate::stream_mod::{
    ChainType, GenericDataProto, GetBlocksRequest, streamout_client::StreamoutClient,
};
use massbit_common::NetworkType;
use crate::postgres_adapter::{PostgresAdapterBuilder, PostgresAdapter};
use crate::storage_adapter::StorageAdapter;

pub const GET_STREAM_TIMEOUT_SEC: u64 = 60;
pub const GET_BLOCK_TIMEOUT_SEC: u64 = 600;
pub const DEFAULT_DATABASE_URL : &str = "postgres://graph-node:let-me-in@localhost/analytic";

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
    client: &mut StreamoutClient<Timeout<Channel>>,
    chain_type: ChainType,
    start_block: u64,
    network: &Option<NetworkType>,
) -> Option<Streaming<GenericDataProto>> {
    log::info!("Create new stream from block {}", start_block);
    let get_blocks_request = GetBlocksRequest {
        start_block_number: start_block,
        end_block_number: 0,
        chain_type: chain_type as i32,
        network: network.clone().unwrap_or(Default::default()),
    };
    match client
        .list_blocks(Request::new(get_blocks_request.clone()))
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

pub fn get_block_number(conn: &PgConnection, chain_value: String, network_value: String) -> Option<NetworkState> {
    use crate::schema::network_state::dsl::*;
    let results = network_state.filter(chain.eq(chain_value))
        .filter(network.eq(network_value))
        .limit(1)
        .load::<NetworkState>(conn)
        .expect("Error loading network state");
    if results.len() == 0 {
        None
    } else {
        match results.get(0) {
            Some(val) => Some(val.clone()),
            None => None
        }
    }
}
