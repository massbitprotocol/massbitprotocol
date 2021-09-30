use clap::{App, Arg};

use graph::ipfs_client::IpfsClient;
use graph_core::LinkResolver;
use log::{debug, info, warn};
use massbit::firehose::dstream::{
    streamout_client::StreamoutClient, ChainType, DataType, GenericDataProto, GetBlocksRequest,
};
use massbit_chain_ethereum::data_type::{decode as ethereum_decode, get_events, EthereumBlock};
use massbit_chain_solana::data_type::{
    convert_solana_encoded_block_to_solana_block, decode as solana_decode, SolanaEncodedBlock,
    SolanaLogMessages, SolanaTransaction,
};
use massbit_chain_substrate::data_type::{SubstrateBlock, SubstrateEventRecord};

use chain_ethereum::DataSource;
use graph::data::subgraph::{SubgraphAssignmentProviderError, UnresolvedSubgraphManifest};
use graph::log::logger;
use massbit::blockchain::DataSource as _;

use massbit_chain_substrate::data_type::{decode, get_extrinsics_from_block};

use std::error::Error as StdError;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_compat_02::FutureExt;

use serde_yaml::Value;

use chain_ethereum::manifest::resolve_manifest_from_text;
use chain_ethereum::network::{EthereumNetworkAdapter, EthereumNetworkAdapters, EthereumNetworks};
use chain_ethereum::transport::Transport;
use chain_ethereum::{manifest, Chain, EthereumAdapter};
use massbit::blockchain::block_stream::BlockWithTriggers;
use massbit::blockchain::Block as _;
use massbit::blockchain::{Blockchain, TriggerFilter};
use massbit::prelude::*;
use massbit::semver::Version;
#[allow(unused_imports)]
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status,
};

// pub mod stream_mod {
//     tonic::include_proto!("chaindata");
// }

const URL: &str = "http://127.0.0.1:50051";

pub fn decode_block_with_trigger(
    payload: &mut Vec<u8>,
) -> Result<BlockWithTriggers<Chain>, Box<dyn StdError>> {
    let block: BlockWithTriggers<Chain> = serde_json::from_slice(&payload).unwrap();
    Ok(block)
}

/// Parses an Ethereum connection string and returns the network name and Ethereum adapter.
async fn create_ethereum_adapter() -> EthereumAdapter {
    let (transport_event_loop, transport) =
        Transport::new_rpc("https://rpc-mainnet.matic.network", Default::default());

    // If we drop the event loop the transport will stop working.
    // For now it's fine to just leak it.
    std::mem::forget(transport_event_loop);

    chain_ethereum::EthereumAdapter::new(
        "matic".to_string(),
        "https://rpc-mainnet.matic.network",
        transport,
        false,
    )
    .await
}

pub async fn print_blocks(
    mut client: StreamoutClient<Channel>,
    chain_type: ChainType,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let network = "matic".to_string();
    // Load manifest
    let mut manifest = manifest::resolve_manifest_from_text(YAML).await;
    // Create chain
    let chain = Chain {
        eth_adapters: Arc::new(EthereumNetworkAdapters {
            adapters: vec![EthereumNetworkAdapter {
                adapter: Arc::new(create_ethereum_adapter().await),
            }],
        }),
    };
    // Create filter
    let filter = <chain_ethereum::Chain as Blockchain>::TriggerFilter::from_data_sources(
        manifest.data_sources.iter(),
    );
    println!("filter: {:?}", &filter);
    let encoded_filter = serde_json::to_vec(&filter).unwrap();
    // println!("encoded_filter: {:?}", filter);
    // Get start_block_number
    let start_block_number = manifest
        .data_sources
        .iter()
        .map(|data_source| data_source.start_block())
        .min()
        .unwrap_or(0);
    // Create GetBlocksRequest
    // Todo: start_block_numbers is array but now use only one value
    let get_blocks_request = GetBlocksRequest {
        start_block_number: start_block_number as u64,
        end_block_number: 1,
        chain_type: chain_type as i32,
        network,
        filter: encoded_filter,
    };

    //Create stream
    println!("Creating Stream ...");
    let mut stream = Some(
        client
            .list_blocks(Request::new(get_blocks_request))
            .await?
            .into_inner(),
    );

    //Wait for data
    println!("Waiting for data...");
    while let Some(data) = stream.as_mut().unwrap().message().await? {
        let mut data = data as GenericDataProto;
        println!(
            "Received chain: {:?}, data block = {:?}, hash = {:?}, data type = {:?}",
            ChainType::from_i32(data.chain_type).unwrap(),
            data.block_number,
            data.block_hash,
            DataType::from_i32(data.data_type).unwrap()
        );
        match chain_type {
            ChainType::Substrate => {}
            ChainType::Solana => {}
            ChainType::Ethereum => match DataType::from_i32(data.data_type) {
                Some(DataType::Block) => {
                    let block: EthereumBlock = ethereum_decode(&mut data.payload).unwrap();
                    info!(
                        "Received ETHREUM BLOCK with Block number: {}",
                        &block.block.number.unwrap().as_u64()
                    );
                }
                Some(DataType::BlockWithTriggers) => {
                    let block: BlockWithTriggers<Chain> =
                        decode_block_with_trigger(&mut data.payload).unwrap();
                    info!(
                        "Received ETHREUM BlockWithTrigger with Block : {:?}",
                        &block.block.number()
                    );
                }
                _ => {
                    warn!("Not support this type in Ethereum");
                }
            },
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    env_logger::init();
    info!("Waiting for chain-reader");

    let matches = App::new("Client")
        .version("1.0")
        .about("Client for test chain-reader")
        .arg(
            Arg::with_name("type")
                .short("c")
                .long("chain-type")
                .value_name("type")
                .help("Sets chain type")
                .takes_value(true),
        )
        .get_matches();

    let chain_type = matches.value_of("type").unwrap_or("ethereum");
    let client = StreamoutClient::connect(URL).await.unwrap();
    println!("Match {:?}", matches);
    match chain_type {
        "substrate" => {
            info!("Run client: {}", chain_type);
            print_blocks(client, ChainType::Substrate).await?;
        }
        "solana" => {
            info!("Run client: {}", chain_type);
            print_blocks(client, ChainType::Solana).await?;
        }
        _ => {
            info!("Run client: {}", chain_type);
            print_blocks(client, ChainType::Ethereum).await?;
        }
    };

    Ok(())
}

const YAML: &str = "
dataSources:
- kind: ethereum/contract
  mapping:
    abis:
    - file:
        /: /ipfs/Qmabi
      name: Factory
    - file:
        /: /ipfs/Qmabi
      name: ERC20
    - file:
        /: /ipfs/Qmabi
      name: ERC20SymbolBytes
    - file:
        /: /ipfs/Qmabi
      name: ERC20NameBytes
    apiVersion: 0.0.4
    entities:
    - Pair
    - Token
    eventHandlers:
    - event: PairCreated(indexed address,indexed address,address,uint256)
      handler: handleNewPair
    file:
      /: /ipfs/Qmmapping
    kind: ethereum/events
    language: wasm/assemblyscript
  name: Factory
  network: matic
  source:
    abi: Factory
    address: '0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32'
    startBlock: 5484576
description: Quickswap is a decentralized protocol for automated token exchange on
  Matic.
graft:
  base: QmfZAUKkHkLzKtVFQtGqSs4kKch9dfFg5Exs2zG9yNJrTW
  block: 17116542
repository: https://github.com/QuickSwap/QuickSwap-subgraph.git
schema:
  file:
    /: /ipfs/Qmschema
specVersion: 0.0.2
templates:
- kind: ethereum/contract
  mapping:
    abis:
    - file:
        /: /ipfs/Qmabi
      name: Pair
    - file:
        /: /ipfs/Qmabi
      name: Factory
    apiVersion: 0.0.4
    entities:
    - Pair
    - Token 
    eventHandlers:
    - event: Mint(indexed address,uint256,uint256)
      handler: handleMint
    - event: Burn(indexed address,uint256,uint256,indexed address)
      handler: handleBurn
    - event: Swap(indexed address,uint256,uint256,uint256,uint256,indexed address)
      handler: handleSwap
    - event: Transfer(indexed address,indexed address,uint256)
      handler: handleTransfer
    - event: Sync(uint112,uint112)
      handler: handleSync
    file:
      /: /ipfs/Qmmapping
    kind: ethereum/events
    language: wasm/assemblyscript
  name: Pair
  network: matic
  source:
    abi: Pair
";
