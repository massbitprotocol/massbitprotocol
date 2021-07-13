use chain_reader_substrate::substrate_chain;
use tonic::{transport::Server};
use chain_reader_substrate::grpc_stream::stream_mod::{streamout_server::StreamoutServer, ChainType};
use chain_reader_substrate::grpc_stream::StreamService;
use tokio::sync::broadcast;
use lazy_static::lazy_static;
use std::collections::HashMap;
use solana_chain;

struct Config {
    chains: HashMap<ChainType,Vec<ChainConfig>>,
}

struct ChainConfig{
    url: String,
    ws: String,
}



lazy_static! {
    // Load default config
    static ref CONFIG: Config = Config{
        chains: [
            (ChainType::Substrate,ChainConfig{
                url: "0.0.0.0:50051".to_string(),
                ws: "0.0.0.0:50051".to_string(),
            }),
            (ChainType::Solana,ChainConfig{
                url: "https://api.mainnet-beta.solana.com".to_string(),
                ws: "wss://api.mainnet-beta.solana.com".to_string(),
            }),
        ].inter().cloned().collect()
    };
}


pub async fn run() -> Result<(), Box<dyn std::error::Error>>{
    // Broadcast Channel
    //let (chan, _) = broadcast::channel(1024);
    let mut chans = HashMap::new();

    // Spawm thread get_data

    for chain_type in CONFIG.chains.keys{
        let (chan, _) = broadcast::channel(1024);
        // Clone broadcast channel
        let chan_sender = chan.clone();

        match chain_type {
            // Spawn Substrate get_data
            ChainType::Substrate => {
                // Spawn task
                tokio::spawn(async move {
                    substrate_chain::get_block_and_extrinsic(chan_sender).await;
                });
                let chan_sender = chan.clone();
                // Spawn task
                tokio::spawn(async move {
                    substrate_chain::get_event(chan_sender).await;
                });
                // add chan to chans
                chans.insert(ChainType::Substrate,chan);
            },
            ChainType::Solana => {
                // Spawn task
                tokio::spawn(async move {
                    solana_chain::get_block(chan_sender).await;
                });
                // add chan to chans
                chans.insert(ChainType::Substrate,chan);

                continue;
            },
            ChainType::Ethereum => {
                continue;
            },

        }
    }

    // Run StreamoutServer
    let stream_service = StreamService {
        chans: chans
    };

    let addr = CONFIG.url.parse()?;
    Server::builder()
        .add_service(StreamoutServer::new(stream_service))
        .serve(addr)
        .await?;

    // End
    Ok(())
}