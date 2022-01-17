use crate::{
    RELEASES_FOLDER, SCHEMA_FILE_NAME, SO_FOLDER, SO_MAPPING_FILE_NAME,
    SO_UNPACK_INSTRUCTION_FILE_NAME, SRC_FOLDER, SUBGRAPH_FILE_NAME,
};
use anyhow::anyhow;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

pub fn release_indexer(project_dir: &str) -> Result<String, anyhow::Error> {
    let project_dir = PathBuf::from(project_dir);
    let so_mapping_file_path: PathBuf = project_dir.join(SO_FOLDER).join(SO_MAPPING_FILE_NAME);
    let so_unpack_instruction_file_path: PathBuf = project_dir
        .join(SO_FOLDER)
        .join(SO_UNPACK_INSTRUCTION_FILE_NAME);
    let schema_file_path: PathBuf = project_dir.join(SRC_FOLDER).join(SCHEMA_FILE_NAME);
    let manifest_file_path: PathBuf = project_dir.join(SRC_FOLDER).join(SUBGRAPH_FILE_NAME);
    let release_folder = project_dir.join(RELEASES_FOLDER);
    // Create folder
    println!("Release folder: {:?}", &release_folder);
    fs::create_dir_all(&release_folder)?;
    // Create list file:
    let files = vec![
        &so_mapping_file_path,
        &so_unpack_instruction_file_path,
        &schema_file_path,
        &manifest_file_path,
    ];

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
