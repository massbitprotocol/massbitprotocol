use super::model::ListOptions;
use crate::indexer_service::IndexerInfoService;
use crate::model::IndexerData;
use crate::MAX_JSON_BODY_SIZE;
use futures::lock::Mutex;
use massbit::ipfs_client::IpfsClient;
use massbit::slog::Logger;
use massbit_common::prelude::diesel::r2d2::ConnectionManager;
use massbit_common::prelude::diesel::PgConnection;
use massbit_common::prelude::r2d2;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use warp::http::Method;
use warp::{http::StatusCode, multipart::FormData, Filter, Rejection, Reply};

#[derive(Default)]
pub struct ServerBuilder {
    entry_point: String,
    connection_pool: Option<Arc<r2d2::Pool<ConnectionManager<PgConnection>>>>,
    logger: Option<Logger>,
}
pub struct IndexerInfoServer {
    entry_point: String,
    indexer_service: Arc<IndexerInfoService>,
}

impl<'a> IndexerInfoServer {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::default()
    }
    pub async fn serve(&self) {
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec![
                "Access-Control-Allow-Headers",
                "Access-Control-Request-Method",
                "Access-Control-Request-Headers",
                "Origin",
                "Accept",
                "X-Requested-With",
                "Content-Type",
            ])
            .allow_methods(&[
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
                Method::HEAD,
            ]);

        let router = self
            .create_route_indexer_github_deploy(self.indexer_service.clone())
            .with(&cors)
            .or(self
                .create_route_indexer_list(self.indexer_service.clone())
                .with(&cors))
            .or(self
                .create_route_indexer_detail(self.indexer_service.clone())
                .with(&cors))
            .recover(handle_rejection);
        let socket_addr: SocketAddr = self.entry_point.parse().unwrap();

        warp::serve(router).run(socket_addr).await;
    }
    /// Indexer deploy from github api
    fn create_route_indexer_github_deploy(
        &self,
        service: Arc<IndexerInfoService>,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("indexers" / "gitdeploy")
            .and(warp::post())
            .and(json_body())
            .and_then(move |content: IndexerData| {
                let clone_service = service.clone();
                async move { clone_service.deploy_git_indexer(content).await }
            })
    }
    /// Indexer create api
    fn create_route_indexer_create(
        &self,
        service: Arc<IndexerInfoService>,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("indexers")
            .and(warp::post())
            .and(json_body())
            .and_then(move |content: IndexerData| {
                let clone_service = service.clone();
                async move { clone_service.create_indexer(content).await }
            })
    }
    /// Indexer list api
    fn create_route_indexer_list(
        &self,
        service: Arc<IndexerInfoService>,
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
        service: Arc<IndexerInfoService>,
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

    pub fn build(&self) -> IndexerInfoServer {
        IndexerInfoServer {
            entry_point: self.entry_point.clone(),
            indexer_service: Arc::new(IndexerInfoService {
                connection_pool: self.connection_pool.as_ref().unwrap().clone(),
                logger: self.logger.as_ref().unwrap().clone(),
            }),
        }
    }
}
fn json_body() -> impl Filter<Extract = (IndexerData,), Error = warp::Rejection> + Clone {
    // When accepting a body, we want a JSON body
    // (and to reject huge payloads)...
    warp::body::content_length_limit(MAX_JSON_BODY_SIZE).and(warp::body::json())
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
