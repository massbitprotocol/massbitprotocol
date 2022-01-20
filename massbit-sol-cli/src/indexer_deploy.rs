use crate::{
    INDEXER_LOGIC_FOLDER, SCHEMA_FILE_NAME, SO_FOLDER, SO_MAPPING_FILE_NAME,
    SO_UNPACK_INSTRUCTION_FILE_NAME, SRC_FOLDER, SUBGRAPH_FILE_NAME, UNPACK_INSTRUCTION_FOLDER,
};
use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::Client;
use std::path::PathBuf;
use std::process;

pub fn deploy_indexer(indexer_url: &str, project_dir: &str) -> Result<String, anyhow::Error> {
    println!("indexer_url: {}, project_dir: {}", indexer_url, project_dir);
    let project_dir = PathBuf::from(project_dir);
    let so_mapping_file_path: PathBuf = project_dir
        .join(INDEXER_LOGIC_FOLDER)
        .join(SO_FOLDER)
        .join(SO_MAPPING_FILE_NAME);
    let so_unpack_instruction_file_path: PathBuf = project_dir
        .join(UNPACK_INSTRUCTION_FOLDER)
        .join(SO_FOLDER)
        .join(SO_UNPACK_INSTRUCTION_FILE_NAME);
    let schema_file_path: PathBuf = project_dir
        .join(INDEXER_LOGIC_FOLDER)
        .join(SRC_FOLDER)
        .join(SCHEMA_FILE_NAME);
    let manifest_file_path: PathBuf = project_dir
        .join(INDEXER_LOGIC_FOLDER)
        .join(SRC_FOLDER)
        .join(SUBGRAPH_FILE_NAME);
    println!("schema_file_path: {:?}", &schema_file_path);
    println!("manifest_file_path: {:?}", &manifest_file_path);
    println!("so_mapping_file_path: {:?}", &so_mapping_file_path);
    println!("so_unpack_instruction_file_path: {:?}", &schema_file_path);
    let multipart = Form::new()
        .part(
            "unpack-instruction",
            Part::file(so_unpack_instruction_file_path.to_str().unwrap_or_default())?,
        )
        .part(
            "mapping",
            Part::file(so_mapping_file_path.to_str().unwrap_or_default())?,
        )
        .part(
            "schema",
            Part::file(schema_file_path.to_str().unwrap_or_default())?,
        )
        .part(
            "manifest",
            Part::file(manifest_file_path.to_str().unwrap_or_default())?,
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
