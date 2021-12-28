use crate::opt::AccessControl;
use crate::runner::MonitorRunner;
use futures::prelude::*;
use hyper::Server;
use massbit_common::prelude::log::info;
use massbit_common::prelude::slog::Logger;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use thiserror::Error;

/// Common trait for GraphQL server implementations.
pub trait MonitorServerTrait {
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
pub enum MonitorError {
    #[error("Bind error: {0}")]
    BindError(hyper::Error),
}

impl From<hyper::Error> for MonitorError {
    fn from(err: hyper::Error) -> Self {
        MonitorError::BindError(err)
    }
}

/// A GraphQL server based on Hyper.
pub struct MonitorServer<S> {
    logger: Logger,
    runner: Arc<S>,
}

impl<S> MonitorServer<S> {
    /// Creates a new GraphQL server.
    pub fn new(logger: Logger, runner: Arc<S>) -> Self {
        MonitorServer { logger, runner }
    }
}

impl<S> MonitorServerTrait for MonitorServer<S>
where
    S: MonitorRunner,
{
    type ServeError = MonitorError;

    fn serve(
        &mut self,
        port: u16,
        ws_port: u16,
        access_control: AccessControl,
    ) -> Result<Box<dyn Future<Item = (), Error = ()> + Send>, Self::ServeError> {
        let logger = self.logger.clone();

        info!("Starting GraphQL HTTP server at: http://localhost:{}", port);

        let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);

        // On every incoming request, launch a new GraphQL service that writes
        // incoming queries to the query sink.
        let logger_for_service = self.logger.clone();
        let runner = self.runner.clone();
        let new_service = make_service_fn(move |_| {
            futures03::future::ok::<_, Error>(GraphQLService::new(
                logger_for_service.clone(),
                metrics.clone(),
                runner.clone(),
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
