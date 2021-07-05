use std::{path::PathBuf};
use std::error::Error;
use std::fs;
use tokio_compat_02::FutureExt;
use tonic::{Request};

// Massbit dependencies
use ipfs_client::core::create_ipfs_clients;
use massbit_chain_substrate::data_type::SubstrateBlock;
use plugin::manager::PluginManager;
use stream_mod::{GenericDataProto, GetBlocksRequest};
use stream_mod::streamout_client::StreamoutClient;
use crate::types::DeployIpfsParams;
use diesel::{RunQueryDsl, PgConnection, Connection};

pub async fn get_index_config(ipfs_config_hash: &String) -> serde_yaml::Mapping {
    let ipfs_addresses = vec!["0.0.0.0:5001".to_string()];
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await;

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
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await;

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
pub async fn loop_blocks(params: DeployIpfsParams) -> Result<(), Box<dyn Error>> {
    let mut client = StreamoutClient::connect(URL).await.unwrap();
    // Lazily config database connection string, not a good method because this will leak connection to indexer
    let db_connection_string = "postgres://graph-node:let-me-in@localhost";

    // Not use start_block_number start_block_number yet
    let get_blocks_request = GetBlocksRequest{
        start_block_number: 0,
        end_block_number: 1,
    };

    let mut stream = client
        .list_blocks(Request::new(get_blocks_request))
        .await?
        .into_inner();

    log::info!("[Index Manager Helper] Start plugin manager");

    // Get mapping file
    let mapping_file_name = get_index_mapping_file_name(&params.ipfs_mapping_hash).await;

    // Create model based on the user's raw query
    let raw_query = get_raw_query_from_model_hash(&params.ipfs_model_hash).await;
    let query = diesel::sql_query(raw_query);
    let c = PgConnection::establish(db_connection_string).expect(&format!("Error connecting to {}", db_connection_string));
    let result = query.execute(&c);
    match result {
        Ok(_) => {
            log::info!("[Index Manager Helper] Table created successfully");
        },
        Err(_) => {
            log::warn!("[Index Manager Helper] Table already exists");
        }
    }

    // Getting data
    while let Some(block) = stream.message().await? {
        let block = block as GenericDataProto;
        log::info!("[Index Manager Helper] Received block = {:?}, hash = {:?} from {:?}",block.block_number, block.block_hash, params.index_name);

        let mapping_file_location = ["./", &mapping_file_name].join("");
        let library_path = PathBuf::from(mapping_file_location.to_string());
        let mut plugins = PluginManager::new();
        unsafe {
            plugins
                .load(&library_path)
                .expect("plugin loading failed");
        }

        let decode_block: SubstrateBlock = serde_json::from_slice(&block.payload).unwrap();
        log::info!("Decoding block: {:?}", decode_block);

        plugins.handle_block(&String::from(db_connection_string), &decode_block); // Block handling
    }
    Ok(())
}