use clap::{App, Arg};
use core::future::pending;
use massbit_common::cheap_clone::CheapClone;
use massbit_common::prelude::diesel::PgConnection;
use massbit_common::prelude::futures03::compat::Future01CompatExt;
use massbit_common::prelude::prometheus::Registry;
use massbit_common::prelude::{
    async_trait::async_trait,
    slog::{self, info, log, o, Logger},
    tokio,
};
use massbit_common::util::task_spawn;
use massbit_data::indexer::{DeploymentHash, NodeId};
use massbit_data::log::factory::LoggerFactory;
use massbit_data::log::logger;
use massbit_data::metrics::{
    registry::MetricsRegistry, Collector, Counter, Gauge, MetricsRegistry as MetricsRegistryTrait,
    Opts, PrometheusError,
};
use massbit_data::prelude::{
    q, s, LoadManager, ObjectOrInterface, Query, QueryExecutionError, QueryResult,
};
use massbit_data::schema::{ApiSchema, Schema};
use massbit_graphql::config::{Config, Opt};
use massbit_graphql::execution::{ExecutionContext, Query as PreparedQuery, Resolver};
use massbit_graphql::query::{execute_query, QueryExecutionOptions};
use massbit_graphql::store_builder::StoreBuilder;
use massbit_graphql::{
    config, opt,
    runner::GraphQlRunner,
    server::{
        graphql::GraphQlRunner as GraphQlRunnerTrait, GraphQLServer as GraphQLQueryServer,
        GraphQLServerTrait,
    },
    CONNECTION_POOL_SIZE, DATABASE_URL,
};
use massbit_storage_postgres::create_r2d2_connection_pool;
use std::collections::HashMap;
use std::sync::Arc;
use structopt::StructOpt;

#[tokio::main]
async fn main() {
    //let opt = opt::Opt::from_args();
    let opt = read_command_option();
    println!("graphql server");
    // let hash = String::from("q42VCQyR7SA3ivHuq1rhUiEErSotqkHXCocMwrKC13Q");
    // println!("{:?}", hash.as_bytes());
    // Obtain ports to use for the GraphQL server(s)
    let http_port = opt.http_port;
    let ws_port = opt.ws_port;
    let logger = logger(opt.debug);
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
    // let connection_pool =
    //     create_r2d2_connection_pool::<PgConnection>(DATABASE_URL.as_str(), *CONNECTION_POOL_SIZE);
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
}

fn read_command_option() {
    let matches = App::new("massbit-graphql")
        .version("1.0")
        .about("Graphql server for indexer data")
        .arg(
            Arg::with_name("postgres-url")
                .short("db-url")
                .long("postgres-url")
                .value_name("postgres-url")
                .help("Input postgres url")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("connection-pool-size")
                .short("pool-size")
                .long("connection-pool-size")
                .value_name("connection-pool-size")
                .help("Input connection pool size")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("http-port")
                .long("http-port")
                .value_name("http-port")
                .help("Input http port of the service")
                .default_value("8000")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("ws-port")
                .long("ws-port")
                .value_name("ws-port")
                .help("Input ws port of the service")
                .default_value("8001")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .value_name("debug")
                .help("Print debug message or not")
                .default_value("false")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("node-id")
                .long("node-id")
                .value_name("node-id")
                .help("Input node id")
                .default_value("default")
                .takes_value(true),
        )
        .get_matches();
    println!("{:?}", &matches);
    let postgres_url = matches
        .value_of("postgres-url")
        .and_then(|val| Some(String::from(val)));
    let http_port = matches
        .value_of("http-port")
        .and_then(|val| val.parse::<u16>().ok())
        .unwrap_or(8000u16);
    let ws_port = matches
        .value_of("ws-port")
        .and_then(|val| val.parse::<u16>().ok())
        .unwrap_or(8001u16);
    let debug = matches
        .value_of("debug")
        .and_then(|val| val.parse::<bool>().ok())
        .unwrap_or(false);
    let node_id = matches.value_of("node-id").unwrap_or("default").to_string();
    let connection_pool_size: u32 = matches
        .value_of("connection-pool-size")
        .unwrap_or("10")
        .parse()
        .unwrap();
    config::Opt {
        postgres_url,
        config: None,
        connection_pool_size,
        postgres_secondary_hosts: vec![],
        postgres_host_weights: vec![],
        node_id,
        http_port,
        ws_port,
        debug,
    }
}
