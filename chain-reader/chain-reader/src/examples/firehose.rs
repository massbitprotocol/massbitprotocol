use anyhow::Error;
use chain_ethereum::network::{EthereumNetworkAdapter, EthereumNetworkAdapters, EthereumNetworks};
use chain_ethereum::transport::Transport;
use chain_ethereum::{manifest, Chain, EthereumAdapter};
use logger::core::init_logger;
use massbit::blockchain::block_stream::BlockWithTriggers;
use massbit::blockchain::Block as _;
use massbit::blockchain::DataSource as _;
use massbit::blockchain::{Blockchain, TriggerFilter};
use massbit::firehose::dstream::{
    streamout_client::StreamoutClient, ChainType, DataType, GenericDataProto, GetBlocksRequest,
};
use massbit::log::logger;
use massbit::{
    firehose::{
        bstream::BlockResponseV2, bstream::BlocksRequestV2, bstream::ForkStep,
        endpoints::FirehoseEndpoint,
    },
    prelude::{error, info, prost, tokio, warn},
};
use massbit_chain_ethereum::data_type::EthereumBlock;
use prost::Message;
use std::error::Error as StdError;
use std::sync::Arc;
use tonic;
use tonic::Streaming;

const URL: &str = "http://127.0.0.1:50051";

pub fn decode_block_with_trigger(
    payload: &mut Vec<u8>,
) -> Result<BlockWithTriggers<Chain>, Box<dyn StdError>> {
    let block: BlockWithTriggers<Chain> = serde_json::from_slice(&payload).unwrap();
    Ok(block)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let logger = logger(true);
    let res = init_logger(&String::from("firehose"));
    let network = "matic".to_string();
    let chain_type = ChainType::Ethereum;
    // Load manifest
    let mut manifest = manifest::resolve_manifest_from_text(YAML).await;

    // Create filter
    let filter = <chain_ethereum::Chain as Blockchain>::TriggerFilter::from_data_sources(
        manifest.data_sources.iter(),
    );
    let encoded_filter = serde_json::to_vec(&filter).unwrap();
    let mut cursor: Option<String> = None;

    let firehose = Arc::new(FirehoseEndpoint::new("massbit", URL, None).await?);

    println!("connecting to the stream!");
    let mut start_block_number = manifest
        .data_sources
        .iter()
        .map(|data_source| data_source.start_block())
        .min()
        .unwrap_or(0);

    loop {
        let get_blocks_request = GetBlocksRequest {
            start_block_number: start_block_number as u64,
            end_block_number: 1,
            chain_type: chain_type as i32,
            network: network.clone(),
            filter: encoded_filter.clone(),
        };
        let mut stream: Streaming<GenericDataProto> =
            match firehose.clone().stream_data(get_blocks_request).await {
                Ok(s) => s,
                Err(e) => {
                    println!("could not connect to stream! {}", e);
                    continue;
                }
            };

        loop {
            let mut data = match stream.message().await {
                Ok(Some(t)) => t,
                Ok(None) => {
                    println!("stream completed");
                    break;
                }
                Err(e) => {
                    println!("error getting message {}", e);
                    break;
                }
            };

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
                        info!(
                            logger,
                            "Received ETHREUM Block with Block : {}", &data.block_number
                        );
                    }
                    Some(DataType::BlockWithTriggers) => {
                        let block: Result<BlockWithTriggers<Chain>, _> =
                            decode_block_with_trigger(&mut data.payload);

                        match block {
                            Ok(block) => {
                                info!(
                                    logger,
                                    "Received ETHREUM BlockWithTrigger with Block : {:?}",
                                    &block.block.number()
                                );
                                start_block_number = block.block.number();
                            }
                            Err(e) => error!(logger, "Unable to decode {:?}", e),
                        }
                    }
                    _ => {
                        warn!(logger, "Not support this type in Ethereum");
                    }
                },
            }
        }
    }
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
