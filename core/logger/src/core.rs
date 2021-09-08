/**
 *** The file is to setup logger to either:
 *** - write to file
 *** - output to console
 *** The default option if RUST_LOG is not specified is INFO logging
 **/
use crate::helper::{log_to_console, log_to_file, message};
use lazy_static::lazy_static;
use std::env;

lazy_static! {
    static ref RUST_LOG: String = env::var("RUST_LOG").unwrap_or(String::from("info")); // If not specified, assume logging level is INFO
    static ref RUST_LOG_TYPE: String = env::var("RUST_LOG_TYPE").unwrap_or(String::from("console")); // If not specified, assume we're logging to console
}

pub fn init_logger(file_name: &String) -> String {
    /* Logging to file */
    if RUST_LOG_TYPE.to_lowercase().as_str() == "file" {
        log_to_file(file_name, &RUST_LOG);
        return message(&RUST_LOG_TYPE, &RUST_LOG);
    }

    /* Logging to console */
    if RUST_LOG_TYPE.to_lowercase().as_str() == "console" {
        log_to_console(&RUST_LOG);
        return message(&RUST_LOG_TYPE, &RUST_LOG);
    }

    return message(&Default::default(), &RUST_LOG); /* Not logging to anything. This should not reach */
}
