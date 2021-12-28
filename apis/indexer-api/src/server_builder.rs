use super::model::ListOptions;
use super::MAX_UPLOAD_FILE_SIZE;
use crate::config::AccessControl;
use crate::indexer_service::IndexerService;
use crate::model::IndexerData;
use crate::user_managerment::auth::{with_auth, Role};
use crate::MAX_JSON_BODY_SIZE;
use futures::lock::Mutex;
use log::info;
use massbit::ipfs_client::IpfsClient;
use massbit::slog::Logger;
use massbit_common::prelude::diesel::r2d2::ConnectionManager;
use massbit_common::prelude::diesel::PgConnection;
use massbit_common::prelude::r2d2;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use warp::http::Method;
use warp::{http::StatusCode, multipart::FormData, Filter, Rejection, Reply};

#[derive(Default)]
pub struct ServerBuilder {
    entry_point: String,
    ipfs_client: Option<IpfsClient>,
    connection_pool: Option<Arc<r2d2::Pool<ConnectionManager<PgConnection>>>>,
    logger: Option<Logger>,
}
pub struct IndexerServer {
    entry_point: String,
    indexer_service: Arc<IndexerService>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DeployParam {
    pub id: String,
}

impl<'a> IndexerServer {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::default()
    }
    pub async fn serve(&self, access_control: AccessControl) {
        let mut allow_headers: Vec<String> = access_control.get_access_control_allow_headers();
        info!("allow_headers: {:?}", allow_headers);
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(allow_headers)
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
            .create_route_indexer_cli_deploy(self.indexer_service.clone())
            .with(&cors)
            .or(self
                .create_route_indexer_github_deploy(self.indexer_service.clone())
                .with(&cors))
            .or(self
                .create_route_indexer_create(self.indexer_service.clone())
                .with(&cors))
            .or(self
                .create_route_my_indexer_list(self.indexer_service.clone())
                .with(&cors))
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
    /// Indexer deploy from cli api
    fn create_route_indexer_cli_deploy(
        &self,
        service: Arc<IndexerService>,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("indexers" / "deploy")
            .and(warp::post())
            .and(warp::multipart::form().max_length(MAX_UPLOAD_FILE_SIZE.clone()))
            .and_then(move |form: FormData| {
                let clone_service = service.clone();
                async move { clone_service.deploy_indexer_cli(form).await }
            })
    }
    /// Indexer deploy from github api
    fn create_route_indexer_github_deploy(
        &self,
        service: Arc<IndexerService>,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("indexers" / "gitdeploy")
            .and(warp::post())
            .and(with_auth(Role::User))
            .and(json_deploy_body())
            .and_then(move |owner_id: String, content: DeployParam| {
                println!("owner_id: {}", owner_id);
                let clone_service = service.clone();
                async move { clone_service.deploy_git_indexer(owner_id, content).await }
            })
    }
    /// Indexer create api
    fn create_route_indexer_create(
        &self,
        service: Arc<IndexerService>,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("indexers" / "create")
            .and(warp::post())
            .and(with_auth(Role::User))
            .and(json_body())
            .and_then(move |owner_id: String, content: IndexerData| {
                println!("owner_id: {}", &owner_id);
                let clone_service = service.clone();
                async move { clone_service.create_indexer(owner_id, content).await }
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
                async move { clone_service.list_indexer(None, options).await }
            })
    }
    /// My-indexer list api
    fn create_route_my_indexer_list(
        &self,
        service: Arc<IndexerService>,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("my-indexers")
            .and(warp::get())
            .and(with_auth(Role::User))
            .and(warp::query::<ListOptions>())
            .and_then(move |owner_id: String, options: ListOptions| {
                let clone_service = service.clone();
                async move { clone_service.list_indexer(Some(owner_id), options).await }
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
    pub fn with_ipfs_clients(mut self, ipfs_client: IpfsClient) -> Self {
        self.ipfs_client = Some(ipfs_client);
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
        if self.ipfs_client.is_none() {
            panic!("Please config ipfs client")
        }
        let ipfs_client = Arc::new(self.ipfs_client.as_ref().unwrap().clone());
        IndexerServer {
            entry_point: self.entry_point.clone(),
            indexer_service: Arc::new(IndexerService {
                ipfs_client: ipfs_client.clone(),
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

fn json_deploy_body() -> impl Filter<Extract = (DeployParam,), Error = warp::Rejection> + Clone {
    // When accepting a body, we want a JSON body
    // (and to reject huge payloads)...
    warp::body::content_length_limit(MAX_JSON_BODY_SIZE).and(warp::body::json())
}

async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
    let (code, message) = if err.is_not_found() {
        (StatusCode::NOT_FOUND, "Not Found".to_string())
    } else if err.find::<warp::reject::PayloadTooLarge>().is_some() {
        (StatusCode::BAD_REQUEST, "Payload too large".to_string())
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        (
            StatusCode::BAD_REQUEST,
            format!("Authorization error, {:?}", err),
        )
    } else {
        eprintln!("unhandled error: {:?}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error".to_string(),
        )
    };

    Ok(warp::reply::with_status(message, code))
}
