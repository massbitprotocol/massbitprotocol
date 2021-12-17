use massbit_common::cheap_clone::CheapClone;
use massbit_common::prelude::futures03::compat::Future01CompatExt;
use massbit_common::prelude::prometheus::Registry;
use massbit_common::prelude::{slog::info, tokio};
use massbit_common::util::task_spawn;
use massbit_data::indexer::NodeId;
use massbit_data::log::factory::LoggerFactory;
use massbit_data::log::logger;
use massbit_data::metrics::registry::MetricsRegistry;
use massbit_data::prelude::LoadManager;
use massbit_graphql::config::Config;
use massbit_graphql::store_builder::StoreBuilder;
use massbit_graphql::{
    opt,
    runner::GraphQlRunner,
    server::{GraphQLServer as GraphQLQueryServer, GraphQLServerTrait},
};
use std::future::pending;
use std::sync::Arc;
use structopt::StructOpt;

#[tokio::main]
async fn main() {
    let opt = opt::Opt::from_args();
    // Obtain ports to use for the GraphQL server(s)
    let http_port = opt.http_port;
    let ws_port = opt.ws_port;
    let logger = logger(true);
    let node_id =
        NodeId::new(opt.node_id.clone()).expect("Node ID must contain only a-z, A-Z, 0-9, and '_'");
    let config = match Config::load(&logger, &opt.clone().into()) {
        Err(e) => {
            eprintln!("configuration error: {}", e);
            std::process::exit(1);
        }
        Ok(config) => config,
    };
    info!(&logger, "Start graphql HTTP server!");
    // Create a component and subgraph logger factory
    let logger_factory = LoggerFactory::new(logger.clone(), None);
    // Set up Prometheus registry
    let prometheus_registry = Arc::new(Registry::new());
    let metrics_registry = Arc::new(MetricsRegistry::new(
        logger.clone(),
        prometheus_registry.clone(),
    ));
    let store_builder =
        StoreBuilder::new(&logger, &node_id, &config, metrics_registry.cheap_clone()).await;
    //Todo: Read expensive queries from a static file
    let expensive_queries = vec![]; //read_expensive_queries().unwrap();
    let load_manager = Arc::new(LoadManager::new(
        &logger,
        expensive_queries,
        metrics_registry.clone(),
    ));
    let store_manager = store_builder.store_manager().await;
    let arc_store_manager = Arc::new(store_manager);
    let graphql_runner = Arc::new(GraphQlRunner::new(&logger, arc_store_manager, load_manager));
    let graphql_metrics_registry = metrics_registry.clone();
    let mut graphql_server = GraphQLQueryServer::new(
        &logger_factory,
        graphql_metrics_registry,
        graphql_runner.clone(),
    );
    graphql_server
        .serve(http_port, ws_port)
        .expect("Failed to start GraphQL query server")
        .compat()
        .await;
    pending::<()>().await;
}
