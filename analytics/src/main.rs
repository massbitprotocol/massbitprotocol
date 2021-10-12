#[macro_use]
extern crate diesel_migrations;
use analytics::ethereum::process_ethereum_stream;
use analytics::solana::process_solana_stream;
use clap::{App, Arg};
use diesel_migrations::embed_migrations;
use lazy_static::lazy_static;
use log::{error, info};
use std::env;
use std::time::Duration;

use analytics::{
    create_postgres_storage, establish_connection, GET_BLOCK_TIMEOUT_SEC, GET_STREAM_TIMEOUT_SEC,
};
use logger::core::init_logger;
use massbit::firehose::bstream::stream_client::StreamClient;
use std::sync::Arc;
use std::thread::sleep;
#[allow(unused_imports)]
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status,
};
use tower::timeout::Timeout;

lazy_static! {
    static ref CHAIN_READER_URL: String =
        env::var("CHAIN_READER_URL").unwrap_or(String::from("http://127.0.0.1:50051"));
    static ref COMPONENT_NAME: String = String::from("[Analytic]");
}

embed_migrations!("./migrations");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let res = init_logger(&String::from("analytic"));
    info!("{}", res); // Print log output type
    info!("Waiting for chain-reader");
    let matches = App::new("Analytic")
        .version("1.0")
        .about("Service for analytics data")
        .arg(
            Arg::with_name("chain")
                .short("c")
                .long("chain")
                .value_name("chain")
                .help("Input chain type")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("network")
                .short("n")
                .long("network")
                .value_name("network")
                .help("Input network name")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("block")
                .short("b")
                .long("start-block")
                .value_name("block")
                .help("Input start block value")
                .takes_value(true),
        )
        .get_matches();
    {
        let conn = establish_connection();
        match embedded_migrations::run(&conn) {
            Ok(res) => println!("Finished embedded_migration {:?}", &res),
            Err(err) => println!("{:?}", &err),
        };
    }
    let chain_type = matches.value_of("chain").unwrap_or("ethereum");
    let network = matches.value_of("network").unwrap_or("matic");
    let block: Option<u64> = matches.value_of("block").and_then(|val| val.parse().ok());
    info!(
        "Start client for chain {} and network {}",
        chain_type, network
    );
    let storage_adapter = Arc::new(create_postgres_storage());
    loop {
        match Channel::from_static(CHAIN_READER_URL.as_str())
            .connect()
            .await
        {
            Ok(channel) => {
                let timeout_channel =
                    Timeout::new(channel, Duration::from_secs(GET_BLOCK_TIMEOUT_SEC));
                let mut client = StreamClient::new(timeout_channel);
                let network = match matches.value_of("network") {
                    None => None,
                    Some(val) => Some(String::from(val)),
                };
                match chain_type {
                    "solana" => {
                        match process_solana_stream(
                            &mut client,
                            storage_adapter.clone(),
                            network,
                            block,
                        )
                        .await
                        {
                            Err(err) => log::error!("{:?}", &err),
                            Ok(_) => {}
                        }
                    }
                    "substrate" => {
                        //process_substrate_stream(&mut client).await;
                    }
                    _ => {
                        match process_ethereum_stream(
                            &mut client,
                            storage_adapter.clone(),
                            network,
                            block,
                        )
                        .await
                        {
                            Err(err) => log::error!("{:?}", &err),
                            Ok(_) => {}
                        }
                    }
                }
            }
            Err(err) => {
                error!(
                    "Can not connect to chain reader at {:?}, {:?}",
                    CHAIN_READER_URL.as_str(),
                    &err
                );
                sleep(Duration::from_secs(GET_STREAM_TIMEOUT_SEC));
            }
        }
    }
}
