use anyhow::Error;
use tokio;
use web3;

use ethereum::network::{EthereumNetworkAdapter, EthereumNetworkAdapters, EthereumNetworks};
use ethereum::transport::Transport;
use ethereum::{manifest, Chain, EthereumAdapter};
use massbit::blockchain::block_stream::BlockStreamEvent;
use massbit::blockchain::{Block, Blockchain, TriggerFilter};
use massbit::prelude::anyhow;
use massbit::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut manifest = manifest::resolve_manifest_from_text(YAML).await;
    let chain = Chain {
        eth_adapters: Arc::new(EthereumNetworkAdapters {
            adapters: vec![EthereumNetworkAdapter {
                adapter: Arc::new(create_ethereum_adapter().await),
            }],
        }),
    };
    let filter = <ethereum::Chain as Blockchain>::TriggerFilter::from_data_sources(
        manifest.data_sources.iter(),
    );
    let start_blocks = manifest.start_blocks();
    let mut block_stream = chain
        .new_block_stream(start_blocks, Arc::new(filter))
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
    }
}

/// Parses an Ethereum connection string and returns the network name and Ethereum adapter.
async fn create_ethereum_adapter() -> EthereumAdapter {
    let (transport_event_loop, transport) =
        Transport::new_rpc("https://rpc-mainnet.matic.network", Default::default());

    // If we drop the event loop the transport will stop working.
    // For now it's fine to just leak it.
    std::mem::forget(transport_event_loop);

    ethereum::EthereumAdapter::new(
        "matic".to_string(),
        "https://rpc-mainnet.matic.network",
        transport,
    )
    .await
}

const YAML: &str = "
specVersion: 0.0.2
description: Quickswap is a decentralized protocol for automated token exchange on Matic.
repository: https://github.com/QuickSwap/QuickSwap-subgraph.git
schema:
  file: ./schema.graphql
dataSources:
  - kind: ethereum/contract
    name: Factory
    network: matic
    source:
      address: '0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32'
      abi: Factory
      startBlock: 5484576
    mapping:
      kind: ethereum/events
      apiVersion: 0.0.4
      language: wasm/assemblyscript
      file: ./src/mappings/factory.ts
      entities:
        - Pair
        - Token
      abis:
        - name: Factory
          file: ./abis/factory.json
        - name: ERC20
          file: ./abis/ERC20.json
        - name: ERC20SymbolBytes
          file: ./abis/ERC20SymbolBytes.json
        - name: ERC20NameBytes
          file: ./abis/ERC20NameBytes.json
      eventHandlers:
        - event: PairCreated(indexed address,indexed address,address,uint256)
          handler: handleNewPair
templates:
  - kind: ethereum/contract
    name: Pair
    network: matic
    source:
      abi: Pair
    mapping:
      kind: ethereum/events
      apiVersion: 0.0.4
      language: wasm/assemblyscript
      file: ./src/mappings/core.ts
      entities:
        - Pair
        - Token
      abis:
        - name: Pair
          file: ./abis/pair.json
        - name: Factory
          file: ./abis/factory.json
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
";
