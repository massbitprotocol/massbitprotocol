use crate::{SCHEMA_FILE_NAME, SO_FILE_NAME, SO_FOLDER, SRC_FOLDER, SUBGRAPH_FILE_NAME};
use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::Client;
use std::path::PathBuf;
use std::process;

pub fn deploy_indexer(indexer_url: &str, project_dir: &str) -> Result<String, anyhow::Error> {
    let project_dir = PathBuf::from(project_dir);
    let so_file_path: PathBuf = project_dir.join(SO_FOLDER).join(SO_FILE_NAME);
    let schema_file_path: PathBuf = project_dir.join(SRC_FOLDER).join(SCHEMA_FILE_NAME);
    let manifest_file_path: PathBuf = project_dir.join(SRC_FOLDER).join(SUBGRAPH_FILE_NAME);
    let multipart = Form::new()
        .part(
            "mapping",
            Part::file(so_file_path.to_str().unwrap()).unwrap(),
        )
        .part(
            "schema",
            Part::file(schema_file_path.to_str().unwrap()).unwrap(),
        )
        .part(
            "manifest",
            Part::file(manifest_file_path.to_str().unwrap()).unwrap(),
        );

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
