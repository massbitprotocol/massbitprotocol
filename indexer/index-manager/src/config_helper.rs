use ipfs_client::core::create_ipfs_clients;
use tokio_compat_02::FutureExt;
use lazy_static::lazy_static;
use std::{env, path::PathBuf, fs};
use std::fs::File;
use std::io::Read;

lazy_static! {
    static ref IPFS_ADDRESS: String =
        env::var("IPFS_ADDRESS").unwrap_or(String::from("0.0.0.0:5001"));
}

// pub async fn get_index_config(ipfs_config_hash: &String) -> serde_yaml::Mapping {
//     let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
//     let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await; // Refactor to use lazy load
//
//     let file_bytes = ipfs_clients[0]
//         .cat_all(ipfs_config_hash.to_string())
//         .compat()
//         .await
//         .unwrap()
//         .to_vec();
//
//     serde_yaml::from_slice(&file_bytes).unwrap()
// }

pub async fn get_query_ipfs(ipfs_model_hash: &String) -> String {
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

pub async fn get_mapping_ipfs(ipfs_mapping_hash: &String) -> String {
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

pub async fn get_config_ipfs(ipfs_config_hash: &String) -> String {
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

pub fn get_query_local(model_path: &String) -> String {
    let mut raw_query = String::new();
    let mut f = File::open(model_path).expect("Unable to open file");
    f.read_to_string(&mut raw_query)
        .expect("Unable to read string");
    raw_query
}

pub fn get_config_local(config_path: &String) -> String {
    let mut config_file = String::new();
    let mut f = File::open(config_path).expect("Unable to open file");
    f.read_to_string(&mut config_file)
        .expect("Unable to read string");
    config_file
}

pub fn get_mapping_local(mapping_path: &String) -> PathBuf {
    let so_file_path = PathBuf::from(mapping_path.to_string());
    so_file_path
}


pub fn read_config_file(config_file_path: &String) -> serde_yaml::Value {
    let mut project_config_string = String::new();
    let mut f = File::open(config_file_path).expect("Unable to open file"); // Refactor: Config to download config file from IPFS instead of just reading from local
    f.read_to_string(&mut project_config_string)
        .expect("Unable to read string"); // Get raw query
    let project_config: serde_yaml::Value = serde_yaml::from_str(&project_config_string).unwrap();
    project_config
}