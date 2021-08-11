use crate::helper::{default_logging, log_to_file};
use lazy_static::lazy_static;
use log::Level::Info;
use std::env;

/**
*** The file is to setup logger to either:
*** - write to file
*** - output to console
*** The default option if RUST_LOG is not specified is INFO logging
**/
lazy_static! {
    static ref LOG_OPTION: String = env::var("RUST_LOG").unwrap_or(String::from("default")); // If not defined, assume it's INFO
}

pub fn init_logger(file_name: &String) {
    if &*LOG_OPTION == "file" {
        log_to_file(file_name);
    } else if &*LOG_OPTION == "default" {
        default_logging();
    } else {
        env_logger::init();
    }
}
