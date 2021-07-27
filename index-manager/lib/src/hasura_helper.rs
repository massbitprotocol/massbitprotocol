/**
*** Objective of this file is to check and get hasura payload from DDL Gen plugin
**/

// Generic dependencies
use std::fs;
use std::fs::{File};
use std::path::PathBuf;
use std::io::Read;
use lazy_static::lazy_static;

lazy_static! {
    static ref MIGRATION_FOLDER: String = String::from("./migrations");
    static ref HASURA_PAYLOAD: String = String::from("hasura_queries.json");
    static ref COMPONENT_NAME: String = String::from("[Index Manger Hasura Helper]");
}

// Check if there are two folder with the same name in the migration folder
pub fn assert_no_duplicated_index(index_name: &String){
    let paths = fs::read_dir(&*MIGRATION_FOLDER).unwrap();
    let mut flag = 0;
    for path in paths {
        let folder = path.unwrap().file_name().into_string().unwrap();
        if folder.contains(index_name) {
            flag = flag + 1;
        }
        if flag >= 2 {
            panic!("{} Index Name already exists in the folder: {}. Plugin hasura won't run", &*COMPONENT_NAME, &*MIGRATION_FOLDER)
        }
    }
}

// Find the first match folder that contains hasura payload
pub fn get_hasura_payload_folder(index_name: &String) -> PathBuf {
    let paths = fs::read_dir(&*MIGRATION_FOLDER).unwrap(); // List all files in the folder
    let mut res = Default::default();
    for path in paths {
        let f_name = path.as_ref().unwrap().file_name().into_string().unwrap(); // Get all the file name in the folder
        if f_name.contains(index_name) {
            log::info!("{} Found the migration folder of index: {}", &*COMPONENT_NAME, &index_name);
            res = path.unwrap().path();
        };
    };
    res
}

// Find the first hasura payload in the folder
pub fn get_hasura_payload(folder: &PathBuf) -> String {
    let paths = fs::read_dir(folder).unwrap(); // List all files in the folder
    let mut res = Default::default();
    for path in paths {
        let f_name = path.as_ref().unwrap().file_name().into_string().unwrap(); // Get all the file name in the folder
        if f_name.contains(&*HASURA_PAYLOAD) {
            log::info!("{} Found the hasura payload: {}", &*COMPONENT_NAME, &*HASURA_PAYLOAD);
            let mut file = File::open(path.unwrap().path()).unwrap();
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap(); // Getting the file content to string
            res = content;
        };
    };
    log::info!("{} Payload: {}", &*COMPONENT_NAME, &res);
    res
}
