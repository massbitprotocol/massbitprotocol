use std::{path::PathBuf, env};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{Read};
use tokio_compat_02::FutureExt;
use tonic::{Request};
use diesel::{RunQueryDsl, PgConnection, Connection, Queryable, QueryResult};
use diesel::result::DatabaseErrorInformation;
use reqwest::Client;
use serde_json::json;
use postgres::{Connection as PostgreConnection, TlsMode};
use serde::{Deserialize};
use node_template_runtime::Event;
use std::hash::Hash;

// Massbit dependencies
use ipfs_client::core::create_ipfs_clients;
use plugin::manager::PluginManager;
use stream_mod::{HelloRequest, GetBlocksRequest, GenericDataProto, ChainType, DataType, streamout_client::StreamoutClient};
use crate::types::{DeployParams, DeployType, Indexer, DetailParams};
use index_store::core::IndexStore;
use massbit_chain_substrate::data_type::{SubstrateBlock as Block, SubstrateHeader as Header, SubstrateUncheckedExtrinsic as Extrinsic, decode_transactions, decode};

pub async fn get_index_config(ipfs_config_hash: &String) -> serde_yaml::Mapping {
    let ipfs_addresses = vec!["0.0.0.0:5001".to_string()];
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await; // Refactor to use lazy load

    let file_bytes = ipfs_clients[0]
        .cat_all(ipfs_config_hash.to_string())
        .compat()
        .await
        .unwrap()
        .to_vec();

    serde_yaml::from_slice(&file_bytes).unwrap()
}

pub async fn get_index_mapping_file_name(ipfs_mapping_hash: &String) -> String {
    let ipfs_addresses = vec!["0.0.0.0:5001".to_string()];
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
        },
        Err(err) => {
            panic!("[Index Manager Helper] Could not write file to local storage {:#?}", err)
        }
    }
}

pub async fn get_raw_query_from_model_hash(ipfs_model_hash: &String) -> String {
    log::info!("[Index Manager Helper] Downloading Raw Query from IPFS");
    let ipfs_addresses = vec!["0.0.0.0:5001".to_string()];
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

pub mod stream_mod {
    tonic::include_proto!("chaindata");
}
const URL: &str = "http://127.0.0.1:50051";
type EventRecord = system::EventRecord<Event, Hash>;

pub async fn loop_blocks(params: DeployParams) -> Result<(), Box<dyn Error>> {
    let db_connection_string = match env::var("DATABASE_URL") {
        Ok(connection) => connection,
        Err(_) => String::from("postgres://graph-node:let-me-in@localhost")
    };
    let store = IndexStore {
        connection_string: db_connection_string,
    }; // This store will be used by indexers to insert to database

    // Chain Reader Client to subscribe and get latest block
    let (so_file_path, raw_query) = match params.deploy_type {
        DeployType::Local => {
            // Get SO mapping file location
            let mapping_file_name = params.mapping_path;
            let so_file_path = PathBuf::from(mapping_file_name.to_string());

            // Get raw query to create database
            let mut raw_query = String::new();
            let mut f = File::open(&params.model_path).expect("Unable to open file");
            f.read_to_string(&mut raw_query).expect("Unable to read string");

            (so_file_path, raw_query)
        },
        DeployType::Ipfs => {
            // Get SO mapping file location
            let mapping_file_name = get_index_mapping_file_name(&params.mapping_path).await;
            let mapping_file_location = ["./", &mapping_file_name].join("");

            // Get raw query to create database
            let raw_query = get_raw_query_from_model_hash(&params.model_path).await;
            let so_file_path = PathBuf::from(mapping_file_location.to_string());

            (so_file_path, raw_query)
        },
    };

    // Run raw query migration to create new table
    let query = diesel::sql_query(raw_query.clone());
    let c = PgConnection::establish(&store.connection_string).expect(&format!("Error connecting to {}", store.connection_string));
    let result = query.execute(&c);
    let index_detail_table = raw_query.table_name();
    match result {
        Ok(_) => {
            log::info!("[Index Manager Helper] Table {} created successfully", index_detail_table.unwrap());
        },
        Err(e) => {
            log::warn!("[Index Manager Helper] {}", e);
        }
    };

    // Track all tables with hasura
    let gist_body = json!({
        "type": "track_table",
        "args": {
            "schema": "public",
            "name": "",
        }
    });
    let request_url = "http://localhost:8080/v1/query";
    #[allow(unused_variables)]
        let response = Client::new()
        .post(request_url)
        .json(&gist_body)
        .send().compat().await.unwrap();

    // Create indexers table so we can keep track of the indexers status. Refactor: This should be run by migration not by API.
    let mut indexer_create_query = String::new();
    let mut f = File::open("./indexer/migration/indexers.sql").expect("Unable to open file");
    f.read_to_string(&mut indexer_create_query).expect("Unable to read string"); // Get raw query
    let result = diesel::sql_query(indexer_create_query).execute(&c);
    match result {
        Ok(_) => {
            log::info!("[Index Manager Helper] Init table Indexer");
        },
        Err(e) => {
            log::warn!("[Index Manager Helper] {}", e);
        }
    };

    // Read project.yaml config and add indexer info into indexers table. Should refactor to handle file from IPFS
    let mut project_config_string = String::new();
    // let mut f = File::open(&params.config_path).expect("Unable to open file"); // Refactor: Config to download config file from IPFS instead of just reading from local
    let mut f = File::open("./indexer/example/project.yaml").expect("Unable to open file"); // Hard code the path for testing purpose

    f.read_to_string(&mut project_config_string).expect("Unable to read string"); // Get raw query
    let project_config: serde_yaml::Value = serde_yaml::from_str(&project_config_string).unwrap();
    let network_name = project_config["dataSources"][0]["kind"].as_str().unwrap();
    let index_name = project_config["dataSources"][0]["name"].as_str().unwrap();

    // TODO: we need a way to get the newly created table name or just use graphql API
    let add_new_indexer = format!("INSERT INTO indexers(id, name, network) VALUES ('{}','{}','{}');", params.index_name, index_name, network_name);
    let result = diesel::sql_query(add_new_indexer).execute(&c);
    match result {
        Ok(_) => {
            log::info!("[Index Manager Helper] New indexer created");
        },
        Err(e) => {
            log::warn!("[Index Manager Helper] {}", e);
        }
    };

    // Chain Reader Client Configuration to subscribe and get latest block from Chain Reader Server
    let mut client = StreamoutClient::connect(URL).await.unwrap();
    let get_blocks_request = GetBlocksRequest{
        start_block_number: 0,
        end_block_number: 1,
    };
    let mut stream = client
        .list_blocks(Request::new(get_blocks_request))
        .await?
        .into_inner();

    // Subscribe new blocks
    while let Some(data) = stream.message().await? {
        let mut data = data as GenericDataProto;
        log::info!("[Index Manager Helper] Received block = {:?}, hash = {:?} from {:?}",data.block_number, data.block_hash, params.index_name);

        log::info!("[Index Manager Helper] Start plugin manager");
        let mut plugins = PluginManager::new(&store);
        unsafe {
            // plugins.load(&so_file_path).unwrap();
            plugins.load("./target/release/libtest_plugin.so").unwrap();
        }


        match DataType::from_i32(data.data_type) {
            Some(DataType::Block) => {
                let block: Block = decode(&mut data.payload).unwrap();
                println!("Received BLOCK: {:?}", block.header.number);
            },
            // Some error with this type, comeback and implement this later
            // Some(DataType::Event) => {
            //     let event: EventRecord = decode(&mut data.payload).unwrap();
            //     println!("Received EVENT: {:?}", event);
            // },
            Some(DataType::Transaction) => {
                let extrinsics: Vec<Extrinsic> = decode_transactions(&mut data.payload).unwrap();
                println!("Received Extrinsic: {:?}", extrinsics);
            },

            _ => {
                println!("Not support data type: {:?}", &data.data_type);
            }
        }

        // let decode_block: SubstrateBlock = serde_json::from_slice(&block.payload).unwrap();
        // log::info!("[Index Manager Helper] Decoding block: {:?}", decode_block);
        // assert_eq!(plugins.handle_block("test", &decode_block).unwrap(), ());
    }
    Ok(())
}

// Return indexer list
pub async fn list_handler_helper() -> Result<Vec<Indexer>, Box<dyn Error>> {
    let mut client =
        PostgreConnection::connect("postgresql://graph-node:let-me-in@localhost:5432/graph-node", TlsMode::None).unwrap();

    let mut indexers: Vec<Indexer> = Vec::new();
    for row in &client.query("SELECT id, network, name FROM indexers", &[]).unwrap() {
        let indexer = Indexer {
            id: row.get(0),
            network: row.get(1),
            name: row.get(2),
        };
        indexers.push(indexer);
    }
    Ok((indexers))
}

// Comment this function until we have implemented v2 so we'll have data in the indexed detail table
// Query the indexed data (detail)
// pub async fn detail_handler_helper(params: DetailParams) -> Result<Vec<String>, Box<dyn Error>> {
//     let mut client =
//         PostgreConnection::connect("postgresql://graph-node:let-me-in@localhost:5432/graph-node", TlsMode::None).unwrap();
//     let mut indexers: Vec<Indexer> = Vec::new();
//     let mut indexers_clone: Vec<Indexer> = Vec::new();
//     for row in &client.query("SELECT id, network, name, index_data_ref FROM indexers WHERE id=$1 LIMIT 1", &[&params.index_name]).unwrap() {
//         let indexer = Indexer {
//             id: row.get(0),
//             network: row.get(1),
//             name: row.get(2),
//             index_data_ref: row.get(3),
//         };
//         indexers.push(indexer);
//     }
//
//     let index_data_ref = indexers.into_iter().nth(0).unwrap().index_data_ref;
//     let select_all_index_data_query = format!("SELECT * FROM {}", index_data_ref);
//     let rows = &client.query(&select_all_index_data_query, &[]).unwrap();
//
//     let mut temp: String = "".to_string();
//     let mut data: Vec<String> = Vec::new();
//     for (rowIndex, row) in rows.iter().enumerate() {
//         for (colIndex, column) in row.columns().iter().enumerate() {
//             let colType: String = column.type_().to_string();
//
//             if colType == "int4" { //i32
//                 let value: i32 = row.get(colIndex);
//                 temp = format!("{{ '{}':'{}' }}", column.name(), value.to_string());
//                 data.push(temp);
//             }
//             else if colType == "text" {
//             }
//             //TODO: more type support
//             else {
//                 //TODO: raise error
//             }
//         }
//     }
//
//     Ok((data))
// }