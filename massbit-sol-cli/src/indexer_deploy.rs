use massbit::prelude::reqwest::blocking::multipart::{Form, Part};
use massbit::prelude::reqwest::blocking::Client;
use std::process;

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
