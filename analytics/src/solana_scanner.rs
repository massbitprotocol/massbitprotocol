#[macro_use]
extern crate diesel_migrations;
use analytics::solana::process_solana_channel;
use analytics::solana::reader::loop_get_block;
use analytics::{create_postgres_storage, establish_connection};
use clap::{App, Arg};
use log::{error, info};
use logger::core::init_logger;
use massbit::firehose::bstream::ChainType;
use massbit::prelude::Arc;
use massbit_chain_solana::data_type::SolanaFilter;
use solana_client::rpc_client::RpcClient;
use tokio::sync::mpsc;
use tokio::task;

use analytics::solana::SOLANA_URL;
use analytics::solana::SOLANA_WS;

embed_migrations!("./migrations");
const QUEUE_BUFFER: usize = 1024;
const DEFAULT_NETWORK: &str = "mainnet";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let res = init_logger(&String::from("analytic"));
    info!("{}", res); // Print log output type
    info!("Waiting for chain-reader");
    let matches = App::new("Analytic")
        .version("1.0")
        .about("Service for analytics solana data")
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
    let chain_type = ChainType::Solana;
    let network = matches.value_of("network").unwrap_or("mainnet").to_string();
    let block: Option<u64> = matches.value_of("block").and_then(|val| val.parse().ok());
    info!(
        "Start client for chain {:?} and network {}",
        chain_type, network
    );
    let storage_adapter = Arc::new(create_postgres_storage());
    // Decode filter
    let filter: SolanaFilter = SolanaFilter::new(vec![]);
    //let config = CONFIG.get_chain_config(&chain_type, &network).unwrap();
    //let json_rpc_url = config.url.clone();
    let json_rpc_url = SOLANA_URL.clone();
    info!("Init Solana client, url: {}", json_rpc_url);
    info!("Finished init Solana client");
    let client = Arc::new(RpcClient::new(json_rpc_url.clone()));
    let name = "deployment_solana".to_string();
    let (tx, mut rx) = mpsc::channel(QUEUE_BUFFER);
    let start_block = None;
    // Spawn task
    let network_clone = network.clone();
    massbit::spawn_thread(name, move || {
        massbit::block_on(task::unconstrained(async {
            // Todo: add start at save block after restart
            let resp =
                loop_get_block(tx.clone(), &start_block, &network_clone, &client, &filter).await;
            error!("Restart {:?} response {:?}", &chain_type, resp);
        }))
    });
    //Main thread process received blocks
    match process_solana_channel(&mut rx, storage_adapter.clone(), &network, block).await {
        Err(err) => log::error!("{:?}", &err),
        Ok(_) => {}
    }
    Ok(())
}
