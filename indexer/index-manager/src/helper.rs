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

pub async fn get_index_mapping_file(ipfs_mapping_hash: &String) -> String {
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

pub mod stream_mod {
    tonic::include_proto!("streamout");
}
const URL: &str = "http://127.0.0.1:50051";
pub async fn loop_blocks(params: DeployIpfsParams) -> Result<(), Box<dyn Error>> {
    let mut client = StreamoutClient::connect(URL).await.unwrap();

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
    // The main loop, subscribing to Chain Reader Server to get new block
    let mapping_file_name = get_index_mapping_file(&params.ipfs_mapping_hash).await;

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
        log::debug!("Decoding block: {:?}", decode_block);
        plugins.handle_block(&decode_block); // Block handling
    }
    Ok(())
}