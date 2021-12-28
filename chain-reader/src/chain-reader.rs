use chain_reader::command;
use chain_reader::stream_service::StreamService;
use clap::{App, Arg};
use logger::core::init_logger;
use massbit_grpc::firehose::bstream::stream_server::StreamServer;
use tonic::transport::Server;

const QUEUE_BUFFER: usize = 1024;
const URL: &str = "0.0.0.0:";
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let res = init_logger(&String::from("chain-reader"));
    println!("Log output: {}", res); // Print log output type

    // CLI argument
    let matches = App::new("Chain-reader")
        .version("1.0")
        .about("GRPC server for streaming blocks")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("port")
                .help("Sets port")
                .takes_value(true),
        )
        .get_matches();
    let port = matches.value_of("port").unwrap_or("50051").to_string();
    //command::run().await
    // Rpc server: listens incoming request from indexer.
    // For each indexer create a channel
    // and then filtered data is sent via this channel
    // Init StreamService
    // Run StreamoutServer
    let stream_service = StreamService::new();
    let addr = (URL.to_owned() + &port).parse()?;
    Server::builder()
        .add_service(StreamServer::new(stream_service))
        .serve(addr)
        .await?;

    // End
    Ok(())
}
