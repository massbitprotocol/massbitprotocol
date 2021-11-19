use multipart::client::lazy::Multipart;
use multipart::server::nickel::nickel::hyper::header::ContentType;
use multipart::server::nickel::nickel::hyper::mime;
use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::Client;
use reqwest::header::HeaderMap;
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const SO_FILE_NAME: &str = "libblock.so";
const SCHEMA_FILE_NAME: &str = "schema.graphql";
const SUBGRAPH_FILE_NAME: &str = "subgraph.yaml";
pub fn deploy_indexer(indexer_url: &str, project_dir: &str) -> Result<String, anyhow::Error> {
    let so_file_path = format!("{}/{}/{}", project_dir, "target/release", SO_FILE_NAME);
    let schema_file_path = format!("{}/{}/{}", project_dir, "src", SCHEMA_FILE_NAME);
    let manifest_file_path = format!("{}/{}/{}", project_dir, "src", SUBGRAPH_FILE_NAME);
    let multipart = Form::new()
        .part("mapping", Part::file(so_file_path.as_str()).unwrap())
        .part("schema", Part::file(schema_file_path.as_str()).unwrap())
        .part("manifest", Part::file(manifest_file_path.as_str()).unwrap());

    // Compose a request
    let client = Client::new();
    let request_builder = client.post(&String::from(indexer_url)).multipart(multipart);

    // Send request
    match request_builder.send() {
        Ok(r) => {
            println!("{:#?}", r);
        }
        Err(e) => {
            println!("error: {:?}", e);
            process::exit(0);
        }
    };
    Ok(String::from("Success!"))
}
