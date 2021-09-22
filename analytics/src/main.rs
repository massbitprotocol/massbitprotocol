#[macro_use]
extern crate diesel_migrations;
use clap::{App, Arg};
use diesel_migrations::embed_migrations;
use analytics::ethereum::process_ethereum_block;
//use analytics::solana::process_solana_block;
//use analytics::substrate::process_substrate_block;
use lazy_static::lazy_static;
use log::{info, error};
use std::time::Duration;
use std::env;

#[allow(unused_imports)]
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status,
};
use tower::timeout::Timeout;
use analytics::stream_mod::streamout_client::StreamoutClient;
use analytics::{establish_connection, GET_BLOCK_TIMEOUT_SEC, GET_STREAM_TIMEOUT_SEC};
use std::thread::sleep;

lazy_static! {
    static ref CHAIN_READER_URL: String =
        env::var("CHAIN_READER_URL").unwrap_or(String::from("http://127.0.0.1:50051"));
    static ref COMPONENT_NAME: String = String::from("[Analytic]");
}

embed_migrations!("./migrations");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    env_logger::init();
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
        ).arg(
        Arg::with_name("network")
            .short("n")
            .long("network")
            .value_name("network")
            .help("Input network name")
            .takes_value(true),
        ).arg(
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
            Err(err) => println!("{:?}", &err)
        };
    }
    let chain_type = matches.value_of("chain").unwrap_or("ethereum");
    let network = matches.value_of("network").unwrap_or("matic");
    let block = matches.value_of("block").unwrap_or("0");
    let start_block: u64 = block.parse().unwrap_or_default();
    println!("{}", start_block);
    info!("Start client for chain {} and network {}", chain_type, network);
    loop {
        match Channel::from_static(CHAIN_READER_URL.as_str())
            .connect()
            .await {
            Ok(channel) => {
                let timeout_channel =
                    Timeout::new(channel, Duration::from_secs(GET_BLOCK_TIMEOUT_SEC));
                let mut client = StreamoutClient::new(timeout_channel);
                match chain_type {
                    "solana" => {
                        //process_solana_block(&client).await;
                    },
                    "substrate" => {
                        //process_substrate_block(&client).await;
                    },
                    _ => {
                        let network = match matches.value_of("network") {
                            None => None,
                            Some(val) => Some(String::from(val))
                        };
                        match process_ethereum_block(&mut client, &network, start_block).await {
                            Err(err) => log::error!("{:?}", &err),
                            Ok(_) => {}
                        }
                    }
                }
            }
            Err(err) => {
                error!("Can not connect to chain reader at {:?}, {:?}", CHAIN_READER_URL.as_str(), &err);
                sleep(Duration::from_secs(GET_STREAM_TIMEOUT_SEC));
            }
        }
    }
}
