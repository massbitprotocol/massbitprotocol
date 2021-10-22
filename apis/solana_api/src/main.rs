use jsonrpc_http_server::ServerBuilder;
use logger::core::init_logger;
use solana_api::rpc_handler::create_solana_api_io;

#[tokio::main]
async fn main() {
    let _res = init_logger(&String::from("solana-api"));
    let socket_addr = solana_api::API_ENDPOINT.parse().unwrap();
    let api_io = create_solana_api_io(solana_api::SOLANA_CLIENT.clone());
    let server = ServerBuilder::new(api_io)
        // .request_middleware(|request: hyper::Request<hyper::Body>| {
        //     if request.uri() == "/status" {
        //         Response::ok("Server running OK.").into()
        //     } else {
        //         request.into()
        //     }
        // })
        .threads(1) //Use default
        .start_http(&socket_addr)
        .unwrap();
    log::info!("Solana is started. Ready for request processing...");
    server.wait();
}
