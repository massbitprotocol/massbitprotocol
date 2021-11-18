use diesel::PgConnection;
use jsonrpc_core::IoHandler;
use jsonrpc_http_server::ServerBuilder;
use logger::core::init_logger;
use massbit_store_postgres::helper::create_r2d2_connection_pool;

use indexer_api::api::{RpcIndexers, RpcIndexersImpl};
use indexer_api::{CONNECTION_POOL_SIZE, DATABASE_URL};

#[tokio::main]
async fn main() {
    let _res = init_logger(&String::from("indexer-api"));
    let socket_addr = indexer_api::API_ENDPOINT.parse().unwrap();
    let api_io = create_indexer_api_io();
    let server = ServerBuilder::new(api_io)
        .threads(1) //Use default
        .start_http(&socket_addr)
        .unwrap();
    log::info!("Indexer is started. Ready for request processing...");
    server.wait();
}

pub fn create_indexer_api_io() -> IoHandler {
    let mut io = IoHandler::default();
    //let mut io = MetaIoHandler::with_middleware(MyMiddleware::default());
    let connection_pool =
        create_r2d2_connection_pool::<PgConnection>(DATABASE_URL.as_str(), *CONNECTION_POOL_SIZE);

    let rpc_indexer = RpcIndexersImpl::new(connection_pool.clone());
    io.extend_with(rpc_indexer.to_delegate());
    io
}
