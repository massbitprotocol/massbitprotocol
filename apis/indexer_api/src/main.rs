#[macro_use]
extern crate diesel_migrations;

use diesel::PgConnection;
use diesel_migrations::embed_migrations;
use indexer_api::server_builder::ServerBuilder;
use indexer_api::{COMPONENT_NAME, CONNECTION_POOL_SIZE, DATABASE_URL, HASURA_URL, IPFS_ADDRESS};
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
    let ipfs_clients = create_ipfs_clients();
    let socket_addr = indexer_api::API_ENDPOINT.as_str();
    let server = ServerBuilder::default()
        .with_entry_point(socket_addr)
        .with_ipfs_clients(ipfs_clients)
        .with_hasura_url(HASURA_URL.as_str())
        .with_connection_pool(connection_pool)
        .with_logger(logger(false))
        .build();
    //Start all stored indexer
    server.start_indexers().await;
    server.serve().await;
    log::info!("Indexer is started. Ready for request processing...");
}

fn create_ipfs_clients() -> Vec<IpfsClient> {
    // Parse the IPFS URL from the `--ipfs` command line argument
    let address = if IPFS_ADDRESS.starts_with("http://") || IPFS_ADDRESS.starts_with("https://") {
        IPFS_ADDRESS.clone()
    } else {
        format!("http://{}", IPFS_ADDRESS.as_str())
    };
    let ipfs_client = match IpfsClient::new(address.as_str()) {
        Ok(ipfs_client) => ipfs_client,
        Err(e) => {
            log::error!("Failed to create IPFS client {}", e);
            panic!("Could not connect to IPFS");
        }
    };
    vec![ipfs_client]
}
