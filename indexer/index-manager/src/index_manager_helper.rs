use diesel::{Connection, PgConnection};
use lazy_static::lazy_static;
use postgres::{Connection as PostgreConnection, TlsMode};
use std::error::Error;
use std::{env};

// Massbit dependencies
use crate::types::{DeployParams, Indexer};
use crate::builder::{IndexConfigIpfsBuilder};
use crate::hasura::{track_hasura_with_ddl_gen_plugin};
use crate::store::{insert_new_indexer, migrate_with_ddl_gen_plugin, create_indexers_table_if_not_exists};
use crate::ipfs::read_config_file;
use crate::chain_reader::chain_reader_client_start;

lazy_static! {
    static ref CHAIN_READER_URL: String =
        env::var("CHAIN_READER_URL").unwrap_or(String::from("http://127.0.0.1:50051"));
    static ref DATABASE_CONNECTION_STRING: String = env::var("DATABASE_CONNECTION_STRING")
        .unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
    static ref IPFS_ADDRESS: String =
        env::var("IPFS_ADDRESS").unwrap_or(String::from("0.0.0.0:5001"));
}

pub async fn loop_blocks(params: DeployParams) -> Result<(), Box<dyn Error>> {
    // Get user index mapping logic, query for migration and index's configurations
    let index_config = IndexConfigIpfsBuilder::default()
        .config(params.config).await
        .mapping(params.mapping).await
        .schema(params.schema).await
        .build();

    let connection = PgConnection::establish(&DATABASE_CONNECTION_STRING).expect(&format!(
        "Error connecting to {}",
        *DATABASE_CONNECTION_STRING
    ));

    // Parse config file
    let config = read_config_file(&index_config.config);

    // migrate_with_ddl_gen_plugin(&params.index_name, &index_config.schema, &index_config.config); // Create tables for the new index
    migrate_with_ddl_gen_plugin(&"hard_code_indexer_name".to_string(), &index_config.schema, &index_config.config); // Create tables for the new index
    // track_hasura_with_ddl_gen_plugin(&params.index_name).await; // Track the newly created tables in hasura
    track_hasura_with_ddl_gen_plugin(&"hard_code_indexer_name".to_string()).await;

    create_indexers_table_if_not_exists(&connection); // Create indexers table so we can keep track of the indexers status. TODO: Refactor as part of ddl gen plugin
    // insert_new_indexer(&connection, &params.index_name, &config);  // Create a new indexer so we can keep track of it's status
    insert_new_indexer(&connection, &"hard_code_indexer_name".to_string(), &config);

    // Chain Reader Client Configuration to subscribe and get latest block from Chain Reader Server
    chain_reader_client_start(&config, &index_config.mapping).await;
    Ok(())
}

// Return indexer list
pub async fn list_handler_helper() -> Result<Vec<Indexer>, Box<dyn Error>> {
    // Create indexers table if it doesn't exists. We should do this with migration at the start.
    let connection = PgConnection::establish(&DATABASE_CONNECTION_STRING).expect(&format!(
        "Error connecting to {}",
        *DATABASE_CONNECTION_STRING
    ));
    create_indexers_table_if_not_exists(&connection);

    // User postgre lib for easy query
    let client =
        PostgreConnection::connect(DATABASE_CONNECTION_STRING.clone(), TlsMode::None).unwrap();
    let mut indexers: Vec<Indexer> = Vec::new();

    for row in &client
        .query("SELECT id, network, name FROM indexers", &[])
        .unwrap()
    {
        let indexer = Indexer {
            id: row.get(0),
            network: row.get(1),
            name: row.get(2),
        };
        indexers.push(indexer);
    }

    Ok(indexers)
}
