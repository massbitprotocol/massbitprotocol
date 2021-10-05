use anyhow::Error;
use serde_json::json;
use std::sync::Arc;
use tokio;
use web3;

use chain_ethereum::network::{EthereumNetworkAdapter, EthereumNetworkAdapters, EthereumNetworks};
use chain_ethereum::transport::Transport;
use chain_ethereum::{manifest, Chain, EthereumAdapter};
use massbit::blockchain::block_stream::BlockStreamEvent;
use massbit::blockchain::{Block, Blockchain, TriggerFilter};
use massbit::components::store::{DeploymentId, DeploymentLocator};
use massbit::prelude::anyhow;
use massbit::prelude::DeploymentHash;
use massbit::prelude::*;
use tokio::task;

#[tokio::main]
async fn main() -> Result<(), Error> {
    graph::spawn_thread("deployment".to_string(), move || {
        graph::block_on(task::unconstrained(async {
            let mut manifest = manifest::resolve_manifest_from_text(YAML).await;
            let chain = Chain {
                eth_adapters: Arc::new(EthereumNetworkAdapters {
                    adapters: vec![EthereumNetworkAdapter {
                        adapter: Arc::new(create_ethereum_adapter().await),
                    }],
                }),
            };
            let filter = <chain_ethereum::Chain as Blockchain>::TriggerFilter::from_data_sources(
                manifest.data_sources.iter(),
            );
            // let filter = <chain_ethereum::Chain as Blockchain>::TriggerFilter::from_data_sources(
            //     Vec::new().iter(),
            // );

            let filter_json = serde_json::to_string(&filter).unwrap();
            let filter = serde_json::from_str(filter_json.as_str()).unwrap();
            //let start_blocks = manifest.start_blocks();
            let start_blocks = vec![1];
            let deployment = DeploymentLocator {
                id: DeploymentId(1),
                hash: DeploymentHash::new("HASH".to_string()).unwrap(),
            };
            let mut block_stream = chain
                .new_block_stream(deployment, start_blocks[0], Arc::new(filter))
                .await
                .unwrap();
            loop {
                let block = match block_stream.next().await {
                    Some(Ok(BlockStreamEvent::ProcessBlock(block))) => block,
                    Some(Err(e)) => {
                        continue;
                    }
                    None => unreachable!("The block stream stopped producing blocks"),
                };
                println!("block.number with trigger: {}", block.block.number());
            }
        }))
    });

    loop {}
    Ok(())
}

/// Parses an Ethereum connection string and returns the network name and Ethereum adapter.
async fn create_ethereum_adapter() -> EthereumAdapter {
    let logger = logger(true);
    let (transport_event_loop, transport) =
        Transport::new_rpc("https://rpc-mainnet.matic.network", Default::default());

    // If we drop the event loop the transport will stop working.
    // For now it's fine to just leak it.
    std::mem::forget(transport_event_loop);

    chain_ethereum::EthereumAdapter::new(
        logger,
        "matic".to_string(),
        "https://rpc-mainnet.matic.network",
        transport,
        false,
    )
    .await
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
