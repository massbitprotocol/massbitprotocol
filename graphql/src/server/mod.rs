pub mod error;
pub mod graphql;
pub mod request;
pub mod service;

use crate::config::AccessControl;
pub use crate::server::graphql::GraphQlRunner;
use crate::server::service::{GraphQLService, GraphQLServiceMetrics};
use futures::prelude::*;
use hyper;
use hyper::service::make_service_fn;
use hyper::Server;
use massbit_common::prelude::futures03::{self, TryFutureExt};
use massbit_common::prelude::{
    anyhow::Error,
    slog::{error, info, Logger},
};
use massbit_data::log::factory::{
    ComponentLoggerConfig, ElasticComponentLoggerConfig, LoggerFactory,
};
use massbit_data::metrics::MetricsRegistry;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use thiserror::Error;

/// Common trait for GraphQL server implementations.
pub trait GraphQLServerTrait {
    type ServeError;

    /// Creates a new Tokio task that, when spawned, brings up the GraphQL server.
    fn serve(
        &mut self,
        port: u16,
        ws_port: u16,
        access_control: AccessControl,
    ) -> Result<Box<dyn Future<Item = (), Error = ()> + Send>, Self::ServeError>;
}

/// Errors that may occur when starting the server.
#[derive(Debug, Error)]
pub enum GraphQLServeError {
    #[error("Bind error: {0}")]
    BindError(hyper::Error),
}

impl From<hyper::Error> for GraphQLServeError {
    fn from(err: hyper::Error) -> Self {
        GraphQLServeError::BindError(err)
    }
}

/// A GraphQL server based on Hyper.
pub struct GraphQLServer<Q> {
    logger: Logger,
    metrics: Arc<GraphQLServiceMetrics>,
    graphql_runner: Arc<Q>,
}

impl<Q> GraphQLServer<Q> {
    /// Creates a new GraphQL server.
    pub fn new(
        logger_factory: &LoggerFactory,
        metrics_registry: Arc<impl MetricsRegistry>,
        graphql_runner: Arc<Q>,
    ) -> Self {
        let logger = logger_factory.component_logger(
            "GraphQLServer",
            Some(ComponentLoggerConfig {
                elastic: Some(ElasticComponentLoggerConfig {
                    index: String::from("graphql-server-logs"),
                }),
            }),
        );
        let metrics = Arc::new(GraphQLServiceMetrics::new(metrics_registry.clone()));
        GraphQLServer {
            logger,
            metrics,
            graphql_runner,
        }
    }
}

impl<Q> GraphQLServerTrait for GraphQLServer<Q>
where
    Q: GraphQlRunner,
{
    type ServeError = GraphQLServeError;

    fn serve(
        &mut self,
        port: u16,
        ws_port: u16,
        access_control: AccessControl,
    ) -> Result<Box<dyn Future<Item = (), Error = ()> + Send>, Self::ServeError> {
        let logger = self.logger.clone();

        info!(
            logger,
            "Starting GraphQL HTTP server at: http://localhost:{}", port
        );

        let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);

        // On every incoming request, launch a new GraphQL service that writes
        // incoming queries to the query sink.
        let logger_for_service = self.logger.clone();
        let graphql_runner = self.graphql_runner.clone();
        let metrics = self.metrics.clone();
        let new_service = make_service_fn(move |_| {
            futures03::future::ok::<_, Error>(GraphQLService::new(
                logger_for_service.clone(),
                metrics.clone(),
                graphql_runner.clone(),
                ws_port,
                access_control.clone(),
            ))
        });

        // Create a task to run the server and handle HTTP requests
        let task = Server::try_bind(&addr.into())?
            .serve(new_service)
            .map_err(move |e| error!(logger, "Server error"; "error" => format!("{}", e)));

        Ok(Box::new(task.compat()))
    }
}
