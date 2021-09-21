#[macro_use]
extern crate diesel_migrations;
use clap::{App, Arg};
use diesel_migrations::embed_migrations;
use diesel_migrations::EmbedMigrations;
use analytics::ethereum::process_ethereum_block;
use analytics::solana::process_solana_block;
use analytics::substrate::process_substrate_block;
use log::{debug, info, warn, error};
use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde_yaml::Value;

#[allow(unused_imports)]
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status,
};
use std::path::PathBuf;
use diesel::{PgConnection, Connection};
use analytics::stream_mod::streamout_client::StreamoutClient;
use analytics::establish_connection;

const URL: &str = "http://127.0.0.1:50051";
embed_migrations!("./migrations");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    env_logger::init();
    info!("Waiting for chain-reader");

    let matches = App::new("Analytic")
        .version("1.0")
        .about("Service for analytics data")
        .arg(
            Arg::with_name("reader-url")
                .short("u")
                .long("reader-url")
                .value_name("url")
                .help("Input reader url")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("chain")
                .short("c")
                .long("chain")
                .value_name("chain")
                .help("Input chain type")
                .takes_value(true),
        ).arg(
        Arg::with_name("network")
            .short("n")
            .long("network")
            .value_name("network")
            .help("Input network name")
            .takes_value(true),
    )
        .get_matches();
    {
        let conn = establish_connection();
        match embedded_migrations::run(&conn) {
            Ok(res) => println!("Finished embedded_migration {:?}", &res),
            Err(err) => println!("{:?}", &err)
        };
    }
    let reader_url = matches.value_of("url").unwrap_or(URL).to_string();
    let chain_type = matches.value_of("chain").unwrap_or("ethereum");
    let network = matches.value_of("network").unwrap_or("matic");
    info!("Start client for chain {} and network {}", chain_type, network);
    match StreamoutClient::connect(reader_url.clone()).await {
        Ok(mut client) => {
            match chain_type {
                "solana" => {
                    process_solana_block(client).await;
                },
                "substrate" => {
                    process_substrate_block(client).await;
                },
                _ => {
                    process_ethereum_block(client, String::from(network)).await;
                }
            }
        },
        Err(err) => {
            error!("Can not connect to chain reader at {:?}, {:?}", &reader_url, &err);
        }
    }
    Ok(())
}
