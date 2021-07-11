use chain_reader_substrate::substrate_chain;
use tonic::{transport::Server};
use chain_reader_substrate::grpc_stream::stream_mod::{streamout_server::StreamoutServer, ChainType};
use chain_reader_substrate::grpc_stream::StreamService;
use tokio::sync::broadcast;
use lazy_static::lazy_static;

struct Config {
    chain_types: Vec<ChainType>,
    url: String,
}

lazy_static! {
    // Load default config
    static ref CONFIG: Config = Config{
        chain_types: vec![ChainType::Substrate, ChainType::Ethereum, ChainType::Solana],
        url: "127.0.0.1:50051".to_string(),
    };
}


pub async fn run() -> Result<(), Box<dyn std::error::Error>>{
    // Broadcast Channel
    let (chan, _) = broadcast::channel(1024);

    // Spawm thread get_data
    let chain_types = CONFIG.chain_types.clone();
    for chain_type in chain_types{
        match chain_type {
            // Spawn Substrate get_data
            ChainType::Substrate => {
                // Clone broadcast channel
                let chan_sender = chan.clone();
                // Spawn task
                tokio::spawn(async move {
                    substrate_chain::get_block_and_extrinsic(chan_sender).await;
                });
                let chan_sender = chan.clone();
                // Spawn task
                tokio::spawn(async move {
                    substrate_chain::get_event(chan_sender).await;
                });

            },
            ChainType::Ethereum => {
                continue;
            },
            ChainType::Solana => {
                continue;
            },

        }
    }

    // Run StreamoutServer
    let stream_service = StreamService {
        chan
    };

    let addr = CONFIG.url.parse()?;
    Server::builder()
        .add_service(StreamoutServer::new(stream_service))
        .serve(addr)
        .await?;

    // End
    Ok(())
}