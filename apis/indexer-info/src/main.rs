#[macro_use]
extern crate diesel_migrations;

use diesel::PgConnection;
use diesel_migrations::embed_migrations;
use indexer_info::server_builder::ServerBuilder;
use indexer_info::{COMPONENT_NAME, CONNECTION_POOL_SIZE, DATABASE_URL};
use logger::core::init_logger;
use massbit::ipfs_client::IpfsClient;
use massbit::log::logger;
use massbit_store_postgres::helper::create_r2d2_connection_pool;

embed_migrations!("./migrations");

#[tokio::main]
async fn main() {
    let _res = init_logger(&COMPONENT_NAME);
    let connection_pool =
        create_r2d2_connection_pool::<PgConnection>(DATABASE_URL.as_str(), *CONNECTION_POOL_SIZE);
    if let Ok(conn) = &connection_pool.get() {
        match embedded_migrations::run(conn) {
            Ok(res) => println!("Finished embedded_migration {:?}", &res),
            Err(err) => println!("{:?}", &err),
        };
    }
    let socket_addr = indexer_info::INFO_ENDPOINT.as_str();
    let mut server = ServerBuilder::default()
        .with_entry_point(socket_addr)
        .with_connection_pool(connection_pool)
        .with_logger(logger(false))
        .build();
    //Start all stored indexer
    server.serve().await;
    log::info!("Indexer is started. Ready for request processing...");
}
