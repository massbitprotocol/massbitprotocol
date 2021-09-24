extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

pub mod index_manager;
pub mod index_manager_helper;

pub mod hasura;
pub mod hasura_helper;

pub mod config;
pub mod config_builder;

pub mod ipfs;
pub mod store;

pub mod adapter;
pub mod ddl_gen;
pub mod type_index;
pub mod type_request;

use std::env;
use lazy_static::lazy_static;
lazy_static! {
    pub static ref DATABASE_CONNECTION_STRING: String = env::var("DATABASE_CONNECTION_STRING")
        .unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
}