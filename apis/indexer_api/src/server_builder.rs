use super::model::ListOptions;
use super::MAX_UPLOAD_FILE_SIZE;
use crate::indexer_service::IndexerService;
use massbit::ipfs_client::IpfsClient;
use massbit::prelude::LoggerFactory;
use massbit::slog::Logger;
use massbit_common::prelude::diesel::r2d2::ConnectionManager;
use massbit_common::prelude::diesel::PgConnection;
use massbit_common::prelude::r2d2;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use warp::{http::StatusCode, multipart::FormData, Filter, Rejection, Reply};

#[derive(Default)]
pub struct ServerBuilder {
    entry_point: String,
    ipfs_clients: Vec<IpfsClient>,
    connection_pool: Option<Arc<r2d2::Pool<ConnectionManager<PgConnection>>>>,
    hasura_url: Option<String>,
    logger: Option<Logger>,
}
pub struct IndexerServer {
    entry_point: String,
    indexer_service: Arc<IndexerService>,
}

impl IndexerServer {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::default()
    }
    pub async fn serve(&self) {
        let service = self.indexer_service.clone();
        // let fn_deploy = move |form: FormData| {
        //     let clone_service = service.clone();
        //     async move { clone_service.deploy_indexer(form).await }
        // };
        // /// Indexer deploy
        // let deploy_route = warp::path!("indexer" / "deploy")
        //     .and(warp::post())
        //     .and(warp::multipart::form().max_length(MAX_UPLOAD_FILE_SIZE.clone()))
        //     .and_then(fn_deploy);

        //let download_route = warp::path("files").and(warp::fs::dir("./files/"));

        let router = self
            .create_route_indexer_deploy(self.indexer_service.clone())
            .or(self.create_route_indexer_list(self.indexer_service.clone()))
            .or(self.create_route_indexer_detail(self.indexer_service.clone()))
            .recover(handle_rejection);
        let socket_addr: SocketAddr = self.entry_point.parse().unwrap();

        warp::serve(router).run(socket_addr).await;
    }
    /// Indexer deploy api
    fn create_route_indexer_deploy(
        &self,
        service: Arc<IndexerService>,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("indexers" / "deploy")
            .and(warp::post())
            .and(warp::multipart::form().max_length(MAX_UPLOAD_FILE_SIZE.clone()))
            .and_then(move |form: FormData| {
                let clone_service = service.clone();
                async move { clone_service.deploy_indexer(form).await }
            })
    }
    /// Indexer list api
    fn create_route_indexer_list(
        &self,
        service: Arc<IndexerService>,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("indexers")
            .and(warp::get())
            .and(warp::query::<ListOptions>())
            .and_then(move |options: ListOptions| {
                let clone_service = service.clone();
                async move { clone_service.list_indexer(options).await }
            })
    }
    /// Indexer detail api
    fn create_route_indexer_detail(
        &self,
        service: Arc<IndexerService>,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("indexers" / String)
            .and(warp::get())
            .and_then(move |hash: String| {
                let clone_service = service.clone();
                async move { clone_service.get_indexer(hash).await }
            })
    }
}
impl ServerBuilder {
    pub fn with_entry_point(mut self, entry_point: &str) -> Self {
        self.entry_point = String::from(entry_point);
        self
    }
    pub fn with_ipfs_clients(mut self, ipfs_client: Vec<IpfsClient>) -> Self {
        self.ipfs_clients = ipfs_client;
        self
    }
    pub fn with_hasura_url(mut self, hasura_url: &str) -> Self {
        self.hasura_url = Some(String::from(hasura_url));
        self
    }
    pub fn with_connection_pool(
        mut self,
        connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    ) -> Self {
        self.connection_pool = Some(Arc::new(connection_pool));
        self
    }
    pub fn with_logger(mut self, logger: Logger) -> Self {
        self.logger = Some(logger);
        self
    }

    pub fn build(&self) -> IndexerServer {
        IndexerServer {
            entry_point: self.entry_point.clone(),
            indexer_service: Arc::new(IndexerService {
                ipfs_clients: self.ipfs_clients.clone(),
                connection_pool: self.connection_pool.as_ref().unwrap().clone(),
                logger_factory: LoggerFactory::new(self.logger.as_ref().unwrap().clone()),
            }),
        }
    }
}

async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
    let (code, message) = if err.is_not_found() {
        (StatusCode::NOT_FOUND, "Not Found".to_string())
    } else if err.find::<warp::reject::PayloadTooLarge>().is_some() {
        (StatusCode::BAD_REQUEST, "Payload too large".to_string())
    } else {
        eprintln!("unhandled error: {:?}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error".to_string(),
        )
    };

    Ok(warp::reply::with_status(message, code))
}
