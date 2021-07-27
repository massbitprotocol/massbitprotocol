/**
*** Objective of this file is to call to IPFS and get the index's information
**/

// Generic dependencies
use tokio_compat_02::FutureExt;
use lazy_static::lazy_static;
use std::{env, fs};
use std::fs::File;
use std::io::Read;

// Massbit dependencies
use ipfs_client::core::create_ipfs_clients;

lazy_static! {
    static ref IPFS_ADDRESS: String =
        env::var("IPFS_ADDRESS").unwrap_or(String::from("0.0.0.0:5001"));
}

pub async fn get_schema_ipfs(hash: &String) -> String {
    log::info!("[Index Manager IPFS] Downloading Schema from IPFS");
    let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await;

    let file_bytes = ipfs_clients[0]
        .cat_all(hash.to_string())
        .compat()
        .await
        .unwrap()
        .to_vec();

    let file_name = ["indexer/generated/", hash, ".graphql"].join("");
    let res = fs::write(&file_name, file_bytes); // Add logger and says that write file successfully

    match res {
        Ok(_) => {
            log::info!("[Index Manager IPFS] Write Schema file to storage successfully");
            file_name
        }
        Err(err) => {
            panic!(
                "[Index Manager IPFS] Could not write Schema file to storage {:#?}",
                err
            )
        }
    }
}

pub async fn get_query_ipfs(ipfs_model_hash: &String) -> String {
    log::info!("[Index Manager IPFS] Downloading Raw Query from IPFS");
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

pub async fn get_mapping_ipfs(hash: &String) -> String {
    let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await; // Refactor to use lazy load

    let file_bytes = ipfs_clients[0]
        .cat_all(hash.to_string())
        .compat()
        .await
        .unwrap()
        .to_vec();

    let file_name = ["indexer/generated/", hash, ".so"].join("");
    let res = fs::write(&file_name, file_bytes); // Add logger and says that write file successfully

    match res {
        Ok(_) => {
            log::info!("[Index Manager IPFS] Write SO file to storage successfully");
            file_name
        }
        Err(err) => {
            panic!(
                "[Index Manager IPFS] Could not write SO file to storage {:#?}",
                err
            )
        }
    }
}

pub async fn get_config_ipfs(hash: &String) -> String {
    let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await; // Refactor to use lazy load

    let file_bytes = ipfs_clients[0]
        .cat_all(hash.to_string())
        .compat()
        .await
        .unwrap()
        .to_vec();

    let file_name = ["indexer/generated/", hash, ".yaml"].join("");
    let res = fs::write(&file_name, file_bytes); // Add logger and says that write file successfully

    match res {
        Ok(_) => {
            log::info!(
                "[Index Manager IPFS] Write project.yaml to storage successfully"
            );
            file_name
        }
        Err(err) => {
            panic!(
                "[Index Manager IPFS] Could not write project.yaml to storage {:#?}",
                err
            )
        }
    }
}

pub fn read_config_file(config_file_path: &String) -> serde_yaml::Value {
    let mut project_config_string = String::new();
    let mut f = File::open(config_file_path).expect("Unable to open file"); // Refactor: Config to download config file from IPFS instead of just reading from local
    f.read_to_string(&mut project_config_string)
        .expect("Unable to read string"); // Get raw query
    let project_config: serde_yaml::Value = serde_yaml::from_str(&project_config_string).unwrap();
    project_config
}
