use clap::{App, Arg};
use diesel_migrations::embed_migrations;
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
const URL: &str = "http://127.0.0.1:50051";
//embed_migrations!("../migrations");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    env_logger::init();
    info!("Waiting for chain-reader");

    let matches = App::new("Analytic")
        .version("1.0")
        .about("Service for analytic data")
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
    let reader_url = matches.value_of("url").unwrap_or(URL).to_string();
    let chain_type = matches.value_of("chain").unwrap_or("ethereum");
    let network = matches.value_of("network").unwrap_or("matic");

    //println!("Match {:?}", matches);
    info!("Start client for chain {} and network {}", chain_type, network);
    //embed_migrations!("../migrations");
    //embedded_migrations::run(&connection);
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

// pub async fn start_client(
//     mut client: StreamoutClient<Channel>,
//     chain_type: ChainType,
//     network: String
// ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
//     // Not use start_block_number start_block_number yet
//     let get_blocks_request = GetBlocksRequest {
//         start_block_number: 0,
//         end_block_number: 1,
//         chain_type: chain_type as i32,
//         network,
//     };
//     let mut stream = client
//         .list_blocks(Request::new(get_blocks_request))
//         .await?
//         .into_inner();
//
//     log::info!("Starting read blocks from stream...");
//     while let Some(data) = stream.message().await? {
//         let mut data = data as GenericDataProto;
//         match chain_type {
//             ChainType::Substrate => {
//                 let now = Instant::now();
//                 match DataType::from_i32(data.data_type) {
//                     Some(DataType::Block) => {
//                         let block: SubstrateBlock = decode(&mut data.payload).unwrap();
//                         info!("Received BLOCK: {:?}", &block.block.header.number);
//                         let extrinsics = get_extrinsics_from_block(&block);
//                         for extrinsic in extrinsics {
//                             //info!("Recieved EXTRINSIC: {:?}", extrinsic);
//                             let string_extrinsic = format!("Recieved EXTRINSIC:{:?}", extrinsic);
//                             info!("{}", string_extrinsic);
//                         }
//                     }
//                     Some(DataType::Event) => {
//                         let event: Vec<SubstrateEventRecord> = decode(&mut data.payload).unwrap();
//                         info!("Received Event: {:?}", event);
//                     }
//
//                     _ => {
//                         warn!("Not support data type: {:?}", &data.data_type);
//                     }
//                 }
//                 let elapsed = now.elapsed();
//                 debug!("Elapsed processing solana block: {:.2?}", elapsed);
//             }
//             ChainType::Solana => {
//
//             }
//             ChainType::Ethereum => match DataType::from_i32(data.data_type) {
//                 Some(DataType::Block) => {
//                     let block: EthereumBlock = ethereum_decode(&mut data.payload).unwrap();
//                     info!(
//                         "Recieved ETHREUM BLOCK with Block number: {}",
//                         &block.block.number.unwrap().as_u64()
//                     );
//
//                 }
//                 _ => {
//                     warn!("Not support this type in Ethereum");
//                 }
//             },
//         }
//     }
//
//     Ok(())
// }

