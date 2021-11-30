use anyhow::anyhow;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

const SO_FILE_NAME: &str = "libblock.so";
const SO_FOLDER: &str = "target/release";

const SCHEMA_FILE_NAME: &str = "schema.graphql";
const SUBGRAPH_FILE_NAME: &str = "subgraph.yaml";
const SRC_FOLDER: &str = "src";

const RELEASES_FOLDER: &str = "releases";

pub fn release_indexer(project_dir: &str) -> Result<String, anyhow::Error> {
    let project_dir = PathBuf::from(project_dir);
    let so_file_path = project_dir.join(SO_FOLDER).join(SO_FILE_NAME);
    let schema_file_path = project_dir.join(SRC_FOLDER).join(SCHEMA_FILE_NAME);
    let manifest_file_path = project_dir.join(SRC_FOLDER).join(SUBGRAPH_FILE_NAME);
    let release_folder = project_dir.join(RELEASES_FOLDER);
    // Create folder
    println!("Release folder: {:?}", &release_folder);
    fs::create_dir_all(&release_folder)?;
    // Create list file:
    let files = vec![&so_file_path, &schema_file_path, &manifest_file_path];

    // Copy file to folder
    for file in files {
        // Check file exist
        if file.exists() {
            println!("Copy file {:?} -> Ok", &file);
        } else {
            println!("File {:?} -> Not found", &file);
            return Err(anyhow!("File not found"));
        }
        // Copy file
        fs::copy(
            &file,
            &release_folder.join(
                &file
                    .file_name()
                    .expect(format!("Cannot get destination file name of {:?}", &file).as_str()),
            ),
        )?;
    }

    Ok(String::from("Success!"))
}
