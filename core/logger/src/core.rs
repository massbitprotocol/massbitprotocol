/**
 *** The file is to setup logger to either:
 *** - write to file
 *** - output to console
 *** The default option if RUST_LOG is not specified is INFO logging
 **/
use crate::helper::{default_logging, log_to_file, message};
use lazy_static::lazy_static;
use log::Level::Info;
use std::env;

lazy_static! {
    static ref RUST_LOG: String = env::var("RUST_LOG").unwrap_or(String::from("info")); // If not specified, assume logging level is INFO
    static ref RUST_LOG_TYPE: String = env::var("RUST_LOG_TYPE").unwrap_or(String::from("console")); // If not specified, assume we're logging to console
}

pub fn init_logger(file_name: &String) -> String {
    if &*RUST_LOG_TYPE.to_lowercase() == "file" {
        log_to_file(file_name, &RUST_LOG);
        return message(&RUST_LOG_TYPE, &RUST_LOG);
    }

    if &*RUST_LOG.to_lowercase() == "info" {
        default_logging();
    } else {
        env_logger::init();
    }
    return message(&RUST_LOG_TYPE, &RUST_LOG);
}
