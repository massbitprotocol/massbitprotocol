use diesel::result::DatabaseErrorInformation;
use diesel::{Connection, PgConnection, QueryResult, Queryable, RunQueryDsl};
use lazy_static::lazy_static;
use node_template_runtime::Event;
use postgres::{Connection as PostgreConnection, TlsMode};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use sp_core::{sr25519, H256 as Hash};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::{env, path::PathBuf};
use tokio_compat_02::FutureExt;
use tonic::Request;
use std::time::Instant;
use std::rc::Rc;

// Massbit dependencies
use crate::types::{DeployParams, DeployType, DetailParams, Indexer};
use index_store::core::IndexStore;
use ipfs_client::core::create_ipfs_clients;
use plugin::manager::PluginManager;
use stream_mod::{HelloRequest, GetBlocksRequest, GenericDataProto, ChainType, DataType, streamout_client::StreamoutClient};

// Refactor to new files for substrate / solana
use massbit_chain_substrate::data_type::{decode, decode_transactions, SubstrateBlock as Block, SubstrateBlock, SubstrateHeader as Header, SubstrateUncheckedExtrinsic as Extrinsic, get_extrinsics_from_block, SubstrateEventRecord};
use massbit_chain_solana::data_type::{SolanaBlock, decode as solana_decode, SolanaEncodedBlock, convert_solana_encoded_block_to_solana_block, SolanaTransaction, SolanaLogMessages};
use std::sync::Arc;
use crate::config_reader::IndexConfigBuilder;

// Configs
pub mod stream_mod {
    tonic::include_proto!("chaindata");
}

lazy_static! {
    static ref CHAIN_READER_URL: String =
        env::var("CHAIN_READER_URL").unwrap_or(String::from("http://127.0.0.1:50051"));
    static ref HASURA_URL: String =
        env::var("HASURA_URL").unwrap_or(String::from("http://localhost:8080/v1/query"));
    static ref DATABASE_CONNECTION_STRING: String = env::var("DATABASE_CONNECTION_STRING")
        .unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
    static ref IPFS_ADDRESS: String =
        env::var("IPFS_ADDRESS").unwrap_or(String::from("0.0.0.0:5001"));
}

pub async fn get_index_config(ipfs_config_hash: &String) -> serde_yaml::Mapping {
    let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await; // Refactor to use lazy load

    let file_bytes = ipfs_clients[0]
        .cat_all(ipfs_config_hash.to_string())
        .compat()
        .await
        .unwrap()
        .to_vec();

    serde_yaml::from_slice(&file_bytes).unwrap()
}

pub async fn get_mapping_file_from_ipfs(ipfs_mapping_hash: &String) -> String {
    let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await; // Refactor to use lazy load

    let file_bytes = ipfs_clients[0]
        .cat_all(ipfs_mapping_hash.to_string())
        .compat()
        .await
        .unwrap()
        .to_vec();

    let file_name = [ipfs_mapping_hash, ".so"].join("");
    let res = fs::write(&file_name, file_bytes); // Add logger and says that write file successfully

    match res {
        Ok(_) => {
            log::info!("[Index Manager Helper] Write SO file to local storage successfully");
            file_name
        }
        Err(err) => {
            panic!(
                "[Index Manager Helper] Could not write file to local storage {:#?}",
                err
            )
        }
    }
}

pub async fn get_config_file_from_ipfs(ipfs_config_hash: &String) -> String {
    let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await; // Refactor to use lazy load

    let file_bytes = ipfs_clients[0]
        .cat_all(ipfs_config_hash.to_string())
        .compat()
        .await
        .unwrap()
        .to_vec();

    let file_name = [ipfs_config_hash, ".yaml"].join("");
    let res = fs::write(&file_name, file_bytes); // Add logger and says that write file successfully

    match res {
        Ok(_) => {
            log::info!(
                "[Index Manager Helper] Write project.yaml file to local storage successfully"
            );
            file_name
        }
        Err(err) => {
            panic!(
                "[Index Manager Helper] Could not write file to local storage {:#?}",
                err
            )
        }
    }
}

pub async fn get_raw_query_from_ipfs(ipfs_model_hash: &String) -> String {
    log::info!("[Index Manager Helper] Downloading Raw Query from IPFS");
    let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await;

    let file_bytes = ipfs_clients[0]
        .cat_all(ipfs_model_hash.to_string())
        .compat()
        .await
        .unwrap()
        .to_vec();

    let raw_query = std::str::from_utf8(&file_bytes).unwrap();
    String::from(raw_query)
}

pub fn get_mapping_file_from_local(mapping_path: &String) -> PathBuf {
    let so_file_path = PathBuf::from(mapping_path.to_string());
    so_file_path
}

pub fn get_config_file_from_local(config_path: &String) -> String {
    let mut config_file = String::new();
    let mut f = File::open(config_path).expect("Unable to open file");
    f.read_to_string(&mut config_file)
        .expect("Unable to read string");
    config_file
}

pub fn get_raw_query_from_local(model_path: &String) -> String {
    let mut raw_query = String::new();
    let mut f = File::open(model_path).expect("Unable to open file");
    f.read_to_string(&mut raw_query)
        .expect("Unable to read string");
    raw_query
}

pub fn create_new_indexer_detail_table(connection: &PgConnection, raw_query: &String) {
    let query = diesel::sql_query(raw_query.clone());
    println!("Running: {}", raw_query);
    query.execute(connection);
}

pub fn create_indexers_table_if_not_exists(connection: &PgConnection) {
    let mut query = String::new();
    let mut f = File::open("./indexer/migration/indexers.sql").expect("Unable to open file");
    f.read_to_string(&mut query).expect("Unable to read string"); // Get raw query
    let result = diesel::sql_query(query).execute(connection);
    match result {
        Ok(_) => {
            // log::info!("[Index Manager Helper] Init table Indexer");
        }
        Err(e) => {
            log::warn!("[Index Manager Helper] {}", e);
        }
    };
}

pub fn read_config_file(config_file_path: &String) -> serde_yaml::Value {
    let mut project_config_string = String::new();
    let mut f = File::open(config_file_path).expect("Unable to open file"); // Refactor: Config to download config file from IPFS instead of just reading from local
    f.read_to_string(&mut project_config_string)
        .expect("Unable to read string"); // Get raw query
    let project_config: serde_yaml::Value = serde_yaml::from_str(&project_config_string).unwrap();
    project_config
}

pub fn insert_new_indexer(
    connection: &PgConnection,
    id: &String,
    project_config: &serde_yaml::Value,
) {
    let network = project_config["dataSources"][0]["kind"].as_str().unwrap();
    let name = project_config["dataSources"][0]["name"].as_str().unwrap();

    let add_new_indexer = format!(
        "INSERT INTO indexers(id, name, network) VALUES ('{}','{}','{}');",
        id, name, network
    );
    let result = diesel::sql_query(add_new_indexer).execute(connection);
    match result {
        Ok(_) => {
            log::info!("[Index Manager Helper] New indexer created");
        }
        Err(e) => {
            log::warn!("[Index Manager Helper] {}", e);
        }
    };
}

pub async fn track_hasura_table(table_name: &String) {
    let gist_body = json!({
        "type": "track_table",
        "args": {
            "schema": "public",
            "name": table_name.to_lowercase(),
        }
    });
    Client::new()
        .post(&*HASURA_URL)
        .json(&gist_body)
        .send()
        .compat()
        .await
        .unwrap();
}

pub async fn loop_blocks(params: DeployParams) -> Result<(), Box<dyn Error>> {
    let mut store = IndexStore::new(DATABASE_CONNECTION_STRING.as_str());

    // Get mapping file, raw query to create new table and project.yaml config
    let (mapping_file_path, raw_query, config_file_path) = match params.deploy_type {
        DeployType::Local => {
            let raw_query = get_raw_query_from_local(&params.model_path);
            let mapping_file_path = get_mapping_file_from_local(&params.mapping_path);
            let config_file_path = get_config_file_from_local(&params.config_path);
            (mapping_file_path, raw_query, config_file_path)
        }
        DeployType::Ipfs => {
            let raw_query = get_raw_query_from_ipfs(&params.model_path).await;

            let mapping_file_name = get_mapping_file_from_ipfs(&params.mapping_path).await;
            let mapping_file_location = ["./", &mapping_file_name].join("");
            let mapping_file_path = PathBuf::from(mapping_file_location.to_string());

            let config_file_path = get_config_file_from_ipfs(&params.config_path).await;
            (mapping_file_path, raw_query, config_file_path)
        }
    };

    let indexConfig = IndexConfigBuilder::default()
        .deploy_type(DeployType::Ipfs)
        .query(params.model_path)
        .await
        .build();

    println!("{:?}", indexConfig.query);




    // let connection = PgConnection::establish(&DATABASE_CONNECTION_STRING).expect(&format!(
    //     "Error connecting to {}",
    //     *DATABASE_CONNECTION_STRING
    // ));
    // create_new_indexer_detail_table(&connection, &raw_query);
    //
    // // Track the newly created table with hasura
    // track_hasura_table(&params.table_name).await;
    //
    // // Create indexers table so we can keep track of the indexers status
    // create_indexers_table_if_not_exists(&connection);
    //
    // // Read project.yaml config and add a new indexer row
    // let project_config = read_config_file(&config_file_path);
    // insert_new_indexer(&connection, &params.index_name, &project_config);
    //
    // // Use correct chain type based on config
    // let chain_type = match project_config["dataSources"][0]["kind"].as_str().unwrap() {
    //     "substrate" => ChainType::Substrate,
    //     "solana" => ChainType::Solana,
    //     _ => ChainType::Substrate, // If not provided, assume it's substrate network
    // };
    //
    // // Chain Reader Client Configuration to subscribe and get latest block from Chain Reader Server
    // let mut client = StreamoutClient::connect(CHAIN_READER_URL.clone())
    //     .await
    //     .unwrap();
    // let get_blocks_request = GetBlocksRequest {
    //     start_block_number: 0,
    //     end_block_number: 1,
    //     chain_type: chain_type as i32,
    // };
    // let mut stream = client
    //     .list_blocks(Request::new(get_blocks_request))
    //     .await?
    //     .into_inner();
    //
    // // Subscribe new blocks
    // log::info!("[Index Manager Helper] Start processing block");
    // while let Some(data) = stream.message().await? {
    //     let now = Instant::now();
    //     let mut data = data as GenericDataProto;
    //     log::info!("[Index Manager Helper] Received chain: {:?}, data block = {:?}, hash = {:?}, data type = {:?}",
    //              ChainType::from_i32(data.chain_type).unwrap(),
    //              data.block_number,
    //              data.block_hash,
    //              DataType::from_i32(data.data_type).unwrap());
    //
    //     // Need to refactor this or this will be called every time a new block comes
    //     let mut plugins = PluginManager::new(&mut store);
    //     unsafe {
    //         plugins.load("1234", mapping_file_path.clone()).unwrap();
    //     }
    //
    //     match chain_type {
    //         ChainType::Substrate => {
    //             match DataType::from_i32(data.data_type) {
    //                 Some(DataType::Block) => {
    //                     let block: SubstrateBlock = decode(&mut data.payload).unwrap();
    //                     println!("Received BLOCK: {:?}", &block.block.header.number);
    //                     let extrinsics = get_extrinsics_from_block(&block);
    //                     for extrinsic in extrinsics {
    //                         println!("Received EXTRINSIC: {:?}", extrinsic);
    //                         plugins.handle_substrate_extrinsic("1234", &extrinsic);
    //                     }
    //                     plugins.handle_substrate_block("1234", &block);
    //                 }
    //                 Some(DataType::Event) => {
    //                     let event: SubstrateEventRecord = decode(&mut data.payload).unwrap();
    //                     println!("Received Event: {:?}", event);
    //                     plugins.handle_substrate_event("1234", &event);
    //                 }
    //                 // Some(DataType::Transaction) => {}
    //                 _ => {
    //                     println!("Not support data type: {:?}", &data.data_type);
    //                 }
    //             } // End of Substrate i32 data
    //         } // End of Substrate type
    //         ChainType::Solana => {
    //             match DataType::from_i32(data.data_type) {
    //                 Some(DataType::Block) => {
    //                     let encoded_block: SolanaEncodedBlock = solana_decode(&mut data.payload).unwrap();
    //                     let block = convert_solana_encoded_block_to_solana_block(encoded_block); // Decoding
    //                     //let rc_block = Arc::new(block.clone());
    //                     println!("Received SOLANA BLOCK with block height: {:?}, hash: {:?}", &block.block.block_height.unwrap(), &block.block.blockhash);
    //                     plugins.handle_solana_block("1234", &block);
    //
    //                     let mut print_flag = true;
    //                     for origin_transaction in block.clone().block.transactions {
    //                         let origin_log_messages = origin_transaction.meta.clone().unwrap().log_messages;
    //                         let transaction = SolanaTransaction {
    //                             block_number: ((&block).block.block_height.unwrap() as u32),
    //                             transaction: origin_transaction.clone(),
    //                             //block: rc_block.clone(),
    //                             log_messages: origin_log_messages.clone(),
    //                             success: false
    //                         };
    //
    //                         let log_messages = SolanaLogMessages {
    //                             block_number: ((&block).block.block_height.unwrap() as u32),
    //                             log_messages: origin_log_messages.clone(),
    //                             transaction: origin_transaction.clone(),
    //                         };
    //                         if print_flag {
    //                             //println!("Received Solana transaction & log messages");
    //                             println!("Recieved SOLANA TRANSACTION with Block number: {:?}, transaction: {:?}", &transaction.block_number, &transaction.transaction.transaction.signatures);
    //                             println!("Recieved SOLANA LOG_MESSAGES with Block number: {:?}, log_messages: {:?}", &log_messages.block_number, &log_messages.log_messages.clone().unwrap().get(0));
    //                             print_flag = false;
    //                         }
    //                         plugins.handle_solana_transaction("1234", &transaction);
    //                         plugins.handle_solana_log_messages("1234", &log_messages);
    //                     }
    //                 },
    //                 _ => {
    //                     println!("Not support type in Solana");
    //                 }
    //             } // End of Solana i32 data
    //         }, // End of Solana type
    //         _ => {
    //             println!("Not support this package chain-type");
    //         }
    //     }
    //     let elapsed = now.elapsed();
    //     println!("Elapsed processing block: {:.2?}", elapsed);
    // }
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
    let mut client =
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

    Ok((indexers))
}
