use std::error::Error;
use std::fs;
use std::path::PathBuf;
use tokio_compat_02::FutureExt;
use tonic::Request;
use node_template_runtime;
use node_template_runtime::{DigestItem, Hash, Header};
use std::str::FromStr;
use sp_runtime::Digest;
use store::Store;
use structmap::GenericMap;

// Massbit dependencies
use crate::types::{DeployIpfsParams, DeployLocalParams};
use ipfs_client::core::create_ipfs_clients;
use massbit_chain_substrate::data_type::SubstrateBlock;
use stream_mod::streamout_client::StreamoutClient;
use stream_mod::{GenericDataProto, GetBlocksRequest};
use plugin::PluginManager;
use structmap::value::Value;
use std::ptr::null;
use diesel::{PgConnection, Connection, RunQueryDsl};

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
        }
        Err(err) => {
            panic!(
                "[Index Manager Helper] Could not write file to local storage {:#?}",
                err
            )
        }
    }
}

pub mod stream_mod {
    tonic::include_proto!("chaindata");
}
const URL: &str = "http://127.0.0.1:50051";


fn new_substrate_block() -> SubstrateBlock {
    SubstrateBlock {
        header: Header {
            parent_hash: Hash::from_str(
                "0x5611f005b55ffb1711eaf3b2f5557c788aa2e3d61b1a833f310f9c7e12a914f7",
            )
                .unwrap(),
            number: 610,
            state_root: Hash::from_str(
                "0x173717683ea4459d15d532264aa7c51657cd65d204c033834ffa62f9ea69e78b",
            )
                .unwrap(),
            extrinsics_root: Hash::from_str(
                "0x732ea723e3ff97289d22f2a4a52887329cd37c3b694a4d563979656d1aa6b7ee",
            )
                .unwrap(),
            digest: Digest {
                logs: [DigestItem::ChangesTrieRoot(
                    Hash::from_str(
                        "0x173717683ea4459d15d532264aa7c51657cd65d204c033834ffa62f9ea69e78b",
                    )
                        .unwrap(),
                )]
                    .to_vec(),
            },
        },
        extrinsics: [].to_vec(),
    }
}

// #[derive(FromBTreeMap)]
// struct TestStruct {
//     name: String,
//     value: i32,
// }
//
// impl Default for TestStruct {
//     fn default() -> Self {
//         Self {
//             name: String::new(),
//             value: 0
//         }
//     }
// }

#[derive(Default)]
struct MockStore {}

impl Store for MockStore {
    fn save(&self, _entity_name: String, mut _data: GenericMap) {
        let mut query = format!("INSERT INTO {} (", _entity_name);

        // Compiling the attributes for the insert query
        // Example: INSERT INTO BlockTs (block_hash,block_height)
        for (k, _) in &_data {
            query = format!("{}{},",query, k)
        }
        query = query[0..query.len() - 1].to_string(); // Remove the final `,`
        query = format!("{})",query); // Close the list of attributes

        // Compiling the values for the insert query
        // Example: INSERT INTO BlockTs (block_hash,block_height) VALUES ('0x720c…6c50',610)
        query = format!("{} VALUES (",query); // Add the first `(` for the list of attributes
        for (k, v) in &_data {
            match v.string() {
                Some(r) => {
                    query = format!("{}'{}',",query, r)
                }
                _ => {}
            }
            match v.i64() {
                Some(r) => {
                    query = format!("{}{},",query, r);
                }
                _ => {}
            }
        }
        query = query[0..query.len() - 1].to_string(); // Remove the final `,`
        query = format!("{})",query); // Close the list of attributes
        println!("{}", query); // Inserting the values into the index table

        let connection_string = "postgres://graph-node:let-me-in@localhost";
        let c = PgConnection::establish(&connection_string).expect(&format!("Error connecting to {}", connection_string));
        diesel::sql_query(query).execute(&c);
    }
}

impl MockStore {
    fn new() -> MockStore {
        MockStore::default()
    }
}

pub async fn loop_blocks(params: DeployLocalParams) -> Result<(), Box<dyn Error>> {
    let mapping_file_location = ["./target/release/libtest_plugin.so"].join("");
    let library_path = PathBuf::from(mapping_file_location.to_string());


    let store = MockStore::new();
    let block = new_substrate_block();
    unsafe {
        let mut plugins = PluginManager::new(&store);
        plugins.load(library_path).unwrap();
        // plugins.handle_block("test", &block);
        assert_eq!(plugins.handle_block("test", &block).unwrap(), ());
    }

    Ok(())
}
