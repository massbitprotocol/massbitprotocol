use crate::Transport;
use crate::{
    grpc_stream::stream_mod::{ChainType, DataType, GenericDataProto},
    CONFIG,
};
use anyhow::Error;
use chain_ethereum::{manifest, Chain, EthereumAdapter, TriggerFilter};
use futures::stream;
use futures::{Future, Stream};
use futures03::compat::Future01CompatExt;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use massbit::blockchain::block_stream::{BlockStreamEvent, BlockWithTriggers};
use massbit::blockchain::{Block, Blockchain, TriggerFilter as _};
use massbit::components::store::{DeploymentId, DeploymentLocator};
use massbit::prelude::DeploymentHash;
use massbit::prelude::*;
use massbit_common::NetworkType;
use serde_json::json;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{sleep, timeout, Duration};
use tonic::Status;
use web3;
use web3::transports::Batch;
use web3::{
    types::{
        Block as EthBlock, BlockId, BlockNumber as Web3BlockNumber, Filter, FilterBuilder, Log,
        Transaction, TransactionReceipt, H256,
    },
    Web3,
};

// Check https://github.com/tokio-rs/prost for enum converting in rust protobuf
const CHAIN_TYPE: ChainType = ChainType::Ethereum;
const PULLING_INTERVAL: u64 = 200;
pub(crate) const USE_WEBSOCKET: bool = false;
const BLOCK_BATCH_SIZE: u64 = 10;
const RETRY_GET_BLOCK_LIMIT: u32 = 10;
const GET_BLOCK_TIMEOUT_SEC: u64 = 60;

fn get_web3(network: &NetworkType) -> Arc<Web3<Transport>> {
    let config = CONFIG.get_chain_config(&CHAIN_TYPE, network).unwrap();
    let websocket_url = config.ws.clone();
    let http_url = config.url.clone();

    let (transport_event_loop, transport) = match USE_WEBSOCKET {
        false => Transport::new_rpc(&http_url, Default::default()),
        true => Transport::new_ws(&websocket_url),
    };
    std::mem::forget(transport_event_loop);
    Arc::new(Web3::new(transport))
}

lazy_static! {
    pub static ref WEB3_ETH: Arc<Web3<Transport>> = get_web3(&"ethereum".to_string());
    pub static ref WEB3_BSC: Arc<Web3<Transport>> = get_web3(&"bsc".to_string());
    pub static ref WEB3_MATIC: Arc<Web3<Transport>> = get_web3(&"matic".to_string());
}

#[derive(Error, Debug)]
pub enum IngestorError {
    /// The Ethereum node does not know about this block for some reason, probably because it
    /// disappeared in a chain reorg.
    #[error("Block data unavailable, block was likely uncled (block hash = {0:?})")]
    BlockUnavailable(H256),

    /// An unexpected error occurred.
    #[error("Ingestor error: {0}")]
    Unknown(Error),
}

pub async fn loop_get_block(
    chan: mpsc::Sender<Result<GenericDataProto, Status>>,
    start_block: &Option<u64>,
    network: &NetworkType,
    chain: Arc<Chain>,
    filter: TriggerFilter,
) -> Result<(), Box<dyn StdError>> {
    info!("Start get block {:?}", CHAIN_TYPE);
    info!("Init Ethereum adapter");

    let web3 = chain.eth_adapters.adapters[0].adapter.web3.clone();

    let version = web3
        .net()
        .version()
        .wait()
        .unwrap_or("Cannot get version".to_string());

    // let filter = TriggerFilter::from_data_sources(Vec::new().iter());
    // let filter_json = serde_json::to_string(&filter)?;
    // let filter = serde_json::from_str(filter_json.as_str())?;
    let start_blocks = vec![start_block.unwrap() as i32];
    let deployment = DeploymentLocator {
        id: DeploymentId(1),
        hash: DeploymentHash::new("HASH".to_string()).unwrap(),
    };
    let mut block_stream = chain
        .new_block_stream(deployment, start_blocks[0], Arc::new(filter))
        .await?;
    loop {
        let block = match block_stream.next().await {
            Some(Ok(BlockStreamEvent::ProcessBlock(block))) => block,
            Some(Err(e)) => {
                continue;
            }
            None => unreachable!("The block stream stopped producing blocks"),
        };
        println!("{}", block.block.number());
        let block_hash = block.block.hash().to_string();
        let block_number = block.block.number() as u64;
        // Create generic block
        let generic_block =
            _create_generic_block_with_trigger(block_hash, block_number, &block, version.clone());
        // Send data to GRPC stream
        if !chan.is_closed() {
            let send_res = chan.send(Ok(generic_block as GenericDataProto)).await;
            if send_res.is_ok() {
                info!("gRPC successfully sending block {}", &block_number);
            } else {
                warn!("gRPC unsuccessfully sending block {}", &block_number);
            }
        } else {
            return Err("Stream is closed!".into());
        }
    }
}

fn _create_generic_block_with_trigger(
    block_hash: String,
    block_number: u64,
    block: &BlockWithTriggers<Chain>,
    version: String,
) -> GenericDataProto {
    let generic_data = GenericDataProto {
        chain_type: CHAIN_TYPE as i32,
        version,
        data_type: DataType::BlockWithTriggers as i32,
        block_hash,
        block_number,
        payload: serde_json::to_vec(block).unwrap(),
    };
    // Deserialize
    // let decode_block: BlockWithTriggers<Chain> =
    //     serde_json::from_slice(&generic_data.payload).unwrap();
    // println!("{:?}", decode_block);

    generic_data
}
