use chain_reader::substrate_chain;
use tonic::{transport::Server};
use chain_reader::stream_mod::streamout_server::{StreamoutServer};
use chain_reader::StreamService;
use tokio::sync::broadcast;
use chain_reader::stream_mod::ChainType;
use lazy_static::lazy_static;

struct Config {
    chain_types: Vec<ChainType>,
    url: String,
}

lazy_static! {
    /// The default file size limit for the IPFS cache is 1MiB.
    static ref CONFIG: Config = Config{
        chain_types: vec![ChainType::Substrate, ChainType::Ethereum, ChainType::Solana],
        url: "127.0.0.1:50051".to_string(),
    };
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Broadcast Channel
    let (chan, _) = broadcast::channel(16);

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
                    substrate_chain::get_data(chan_sender).await;
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

