// use jsonrpc_core::futures::{self, future, TryFutureExt};
// use jsonrpc_core::{BoxFuture, IoHandler, Result};
// use jsonrpc_core_client::transports::local;
//use jsonrpc_derive::rpc;
// use jsonrpsee::{
//     http_client::HttpClientBuilder,
//     http_server::{HttpServerBuilder, RpcModule},
//     rpc_params,
//     types::{error::Error, traits::Client},
// };
// use jsonrpsee_http_client::types::JsonValue;
// use jsonrpsee_types::v2::ParamsSer;
//
// use jsonrpc_core::Response;

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
    //.headers(headers)
    // .header(ContentType(mime::Mime(
    //     mime::TopLevel::Multipart,
    //     mime::SubLevel::FormData,
    //     vec![(
    //         mime::Attr::Ext(String::from("boundary")),
    //         mime::Value::Ext(String::from(multipart_prepared.boundary())),
    //     )],
    // )))
    //.body(multipart_buffer);

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
// pub async fn deploy_indexer1(indexer_url: &str, project_dir: &str) -> Result<(), Error> {
//     let client = HttpClientBuilder::default().build(indexer_url)?;
//
//     // let mut map: BTreeMap<&str, JsonValue> = BTreeMap::default();
//     // map.insert("config", json!("config_content"));
//     // map.insert("mapping", json!("mapping_content"));
//     // map.insert("schema", json!("schema_content"));
//     // map.insert("subgraph", json!("subgraph"));
//     // let params = ParamsSer::Map(map);
//     let mapping: Vec<u8> = vec![1, 2, 3];
//     let params = rpc_params!([
//         "config_content",
//         "mapping_content",
//         "schema_content",
//         "subgraph"
//     ]);
//     println!("Params {:?}", &params);
//     let response: Result<Response, _> = client.request("deployIndexer", params).await;
//     log::info!("r: {:?}", response);
//     Ok(())
// }
