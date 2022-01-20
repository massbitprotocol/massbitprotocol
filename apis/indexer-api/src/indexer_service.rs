use crate::git_helper::GitHelper;
use crate::model::{IndexerData, ListOptions};

use crate::server_builder::DeployParam;
use crate::API_LIST_LIMIT;
use crate::INDEXER_MANAGER_DEPLOY_ENDPOINT;
use crate::{
    models::Indexer,
    schema::indexers::{self, dsl},
};
use chain_solana::SolanaIndexerManifest;
use diesel::sql_types::BigInt;
use futures::lock::Mutex;
use indexer_orm::models::IndexerStatus;
use log::debug;
use massbit::ipfs_client::IpfsClient;
use massbit::ipfs_link_resolver::LinkResolver;
use massbit::prelude::prost::bytes::BufMut;
use massbit::prelude::serde_json::Value;
use massbit::prelude::{anyhow, TryStreamExt};
use massbit::slog::Logger;
use massbit_common::prelude::diesel::r2d2::ConnectionManager;
use massbit_common::prelude::diesel::{
    r2d2, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl,
};
use massbit_common::prelude::r2d2::PooledConnection;
use massbit_common::prelude::serde_json::json;
use massbit_common::prelude::tokio::macros::support::Future;
use octocrab::models::activity::Reason;
use std::ops::Deref;
use std::sync::Arc;
use warp::reply::Json;
use warp::{
    multipart::{FormData, Part},
    Rejection, Reply,
};

pub struct IndexerService {
    pub ipfs_client: Arc<IpfsClient>,
    pub connection_pool: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
    pub logger: Logger,
}

impl IndexerService {
    pub fn get_connection(
        &self,
    ) -> Result<
        PooledConnection<ConnectionManager<PgConnection>>,
        massbit_common::prelude::r2d2::Error,
    > {
        self.connection_pool.get()
    }
    pub fn get_indexers(&self) -> Option<Vec<Indexer>> {
        self.get_connection().ok().and_then(|conn| {
            dsl::indexers
                .filter(dsl::deleted.eq(false))
                .load::<Indexer>(conn.deref())
                .ok()
        })
    }
    pub async fn create_indexer(
        &self,
        owner_id: String,
        content: IndexerData,
    ) -> Result<impl Reply, Rejection> {
        log::info!("Create new indexer");
        let mut indexer = Indexer::default();
        indexer.network = content.network;
        indexer.name = content.name.unwrap_or_default();
        indexer.description = content.description;
        indexer.image_url = content.image_url;
        indexer.repository = content.repository;
        indexer.status = IndexerStatus::Draft;
        indexer.owner_id = owner_id;
        indexer.got_block = -1;

        if let Ok(indexer) = self.store_indexer(indexer).await {
            return Ok(warp::reply::json(&json!({"id":indexer.hash})));
        }
        Ok(warp::reply::json(&json!("{'error': 'Cannot store to DB'}")))
    }

    /// for api deploy indexer from massbit-sol cli
    pub async fn deploy_indexer_cli(&self, form: FormData) -> Result<impl Reply, Rejection> {
        log::info!("Deploy new indexer");
        let parts: Vec<Part> = form.try_collect().await.map_err(|e| {
            eprintln!("form error: {}", e);
            warp::reject::reject()
        })?;

        let mut indexer = Indexer::default();
        indexer.got_block = -1_i64;
        indexer.address = Default::default();
        indexer.start_block = Default::default();
        indexer.network = Default::default();
        indexer.name = Default::default();
        for p in parts {
            log::info!("Receive file: {}/{}", &p.name(), p.filename().unwrap());
            let name = format!("{}", &p.name());
            let p_name = name.as_str();
            println!("p_name: {}", p_name);
            match p_name {
                "mapping" | "unpack-instruction" | "schema" | "manifest" => {
                    let value = p
                        .stream()
                        .try_fold(Vec::new(), |mut vec, data| {
                            vec.put(data);
                            async move { Ok(vec) }
                        })
                        .await
                        .map_err(|e| {
                            eprintln!("reading file error: {}", e);
                            warp::reject::reject()
                        })?;

                    match self.ipfs_client.add(value).await {
                        Ok(response) => match p_name {
                            "mapping" => indexer.mapping = response.hash.clone(),
                            "unpack-instruction" => {
                                println!("response: {:?}", &response);
                                indexer.unpack_instruction = response.hash.clone()
                            }
                            "schema" => indexer.graphql = response.hash.clone(),
                            "manifest" => indexer.manifest = response.hash.clone(),
                            &_ => {}
                        },
                        Err(err) => {
                            log::error!("{:?}", &err);
                        }
                    }
                }
                _ => {}
            }
        }
        if let Ok(indexer) = self.store_indexer(indexer).await {
            return self.call_deploy_indexer_manager(indexer).await;
        }
        Ok(warp::reply::json(&json!("{'error': 'Cannot store to DB'}")))
    }

    async fn call_deploy_indexer_manager(
        &self,
        indexer: Indexer,
    ) -> Result<warp::reply::Json, Rejection> {
        // Call Indexer-manager
        log::info!(
            "Call deploy api on Indexer-manager {:?}: {:?}",
            &*INDEXER_MANAGER_DEPLOY_ENDPOINT,
            &indexer
        );
        let res = reqwest::Client::new()
            .post(&*INDEXER_MANAGER_DEPLOY_ENDPOINT)
            .json(&indexer)
            .send()
            .await;
        log::info!("response: {:?}", &res);
        match res {
            Ok(res) => match res.json::<Value>().await {
                Ok(res) => {
                    log::info!("Ok res: {:?}", &res);
                    Ok(warp::reply::json(&res))
                }
                Err(e) => {
                    log::info!("Error: {:?}", &e);
                    Ok(warp::reply::json(&json!({ "error": e.to_string() })))
                }
            },
            Err(e) => return Ok(warp::reply::json(&json!({ "error": e.to_string() }))),
        }
    }

    /// for api deploy indexer from front-end
    pub async fn deploy_git_indexer(
        &self,
        owner_id: String,
        content: DeployParam,
    ) -> Result<impl Reply, Rejection> {
        log::info!("Deploy new indexer from git {:?}.", &content);
        if let Ok(conn) = self.get_connection() {
            let results = dsl::indexers
                .filter(dsl::hash.eq(content.id.as_str()))
                .filter(dsl::status.eq(IndexerStatus::Draft))
                .limit(1)
                .load::<Indexer>(conn.deref())
                .expect("Error loading indexers");
            match results.get(0) {
                Some(res) => {
                    let mut indexer = res.clone();
                    if indexer.owner_id != owner_id {
                        log::error!(
                            "User {} try to deploy indexer {} of owner {}",
                            owner_id,
                            &indexer.hash,
                            &indexer.owner_id
                        );
                        return Ok(warp::reply::json(&json!(
                            "{'error': 'Do not pass authorization check'}"
                        )));
                    }

                    if let Some(git_url) = &indexer.repository {
                        let git_helper = GitHelper::new(git_url);

                        if let Ok(map) = git_helper.load_indexer().await {
                            log::debug!(
                                "Finished load indexer from git, content {:#?}.",
                                &map.keys()
                            );

                            for (file_name, file_content) in map {
                                let values = file_content.to_vec();
                                // Return manifest content for parser
                                match self.ipfs_client.add(values).await {
                                    Ok(response) => match file_name.as_str() {
                                        "mapping" => indexer.mapping = response.hash,
                                        "manifest" => indexer.manifest = response.hash,
                                        "schema" => indexer.graphql = response.hash,
                                        _ => {}
                                    },
                                    Err(err) => {
                                        log::error!("{:?}", &err);
                                    }
                                }
                            }
                        }
                    }
                    debug!("indexer: {:?}", &indexer);
                    // Update indexer status
                    // indexer.status = IndexerStatus::Deploying;
                    if self.update_indexer(&indexer).await.is_ok() {
                        return self.call_deploy_indexer_manager(indexer).await;
                    }
                    Ok(warp::reply::json(&json!("{'error': 'Cannot store to DB'}")))
                }
                None => Ok(warp::reply::json(&json!("{}"))),
            }
        } else {
            Ok(warp::reply::json(&String::from("")))
        }
    }
    async fn store_indexer(&self, mut indexer: Indexer) -> Result<Indexer, anyhow::Error> {
        match self.get_connection() {
            Ok(conn) => {
                indexer.v_id = self.get_next_sequence(&conn, "indexers", "v_id");
                indexer.namespace = format!("sgd{:?}", &indexer.v_id);
                //let indexer = indexer.clone();
                diesel::insert_into(indexers::table)
                    .values(&indexer)
                    .get_result::<Indexer>(&conn)
                    .map_err(|err| anyhow!(format!("{:?}", &err)))
                //.expect("Error while create new indexer");
            }
            Err(err) => Err(anyhow!(format!("{:?}", &err))),
        }
    }
    async fn update_indexer(&self, mut indexer: &Indexer) -> Result<(), anyhow::Error> {
        match self.get_connection() {
            Ok(conn) => {
                if let Err(err) = diesel::update(dsl::indexers.filter(dsl::hash.eq(&indexer.hash)))
                    .set((
                        dsl::namespace.eq(&indexer.namespace),
                        dsl::manifest.eq(&indexer.manifest),
                        dsl::mapping.eq(&indexer.mapping),
                        dsl::graphql.eq(&indexer.graphql),
                        dsl::address.eq(&indexer.address),
                        dsl::start_block.eq(&indexer.start_block),
                        dsl::network.eq(&indexer.network),
                        dsl::version.eq(&indexer.version),
                        dsl::status.eq(&indexer.status),
                    ))
                    .execute(conn.deref())
                {
                    log::error!("{:?}", &err);
                    return Err(anyhow!(format!("{:?}", &err)));
                }
                Ok(())
            }
            Err(err) => Err(anyhow!(format!("{:?}", &err))),
        }
    }
    /// for api list indexer: /indexers?limit=?&offset=?
    pub async fn list_indexer(
        &self,
        owner_id: Option<String>,
        options: ListOptions,
    ) -> Result<impl Reply, Rejection> {
        let content: Vec<Indexer> = vec![];
        if let Ok(conn) = self.get_connection() {
            let res = match owner_id {
                Some(owner_id) => dsl::indexers
                    .filter(dsl::deleted.eq(false))
                    .filter(dsl::owner_id.eq(owner_id))
                    .order(dsl::v_id.asc())
                    .offset(options.offset.unwrap_or_default())
                    .limit(options.limit.unwrap_or(API_LIST_LIMIT))
                    .load::<Indexer>(conn.deref()),
                None => dsl::indexers
                    .filter(dsl::deleted.eq(false))
                    .order(dsl::v_id.asc())
                    .offset(options.offset.unwrap_or_default())
                    .limit(options.limit.unwrap_or(API_LIST_LIMIT))
                    .load::<Indexer>(conn.deref()),
            };
            match res {
                Ok(vals) => Ok(warp::reply::json(&vals)),
                Err(err) => {
                    log::error!("{:?}", &err);
                    Ok(warp::reply::json(&content))
                }
            }
        } else {
            Ok(warp::reply::json(&content))
        }
    }
    /// for api get indexer detail: /indexers/hash
    pub async fn get_indexer(&self, hash: String) -> Result<impl Reply, Rejection> {
        if let Ok(conn) = self.get_connection() {
            let results = dsl::indexers
                .filter(dsl::hash.eq(hash.as_str()))
                .limit(1)
                .load::<Indexer>(conn.deref())
                .expect("Error loading indexers");
            match results.get(0) {
                Some(res) => Ok(warp::reply::json(&res)),
                None => Ok(warp::reply::json(&String::from(""))),
            }
        } else {
            Ok(warp::reply::json(&String::from("")))
        }
    }
    fn get_next_sequence(
        &self,
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        table: &str,
        column: &str,
    ) -> i64 {
        let sql = format!(
            r#"SELECT nextval(pg_get_serial_sequence('{}', '{}')) as value;"#,
            table, column
        );
        #[derive(Debug, Default, QueryableByName)]
        struct SequenceNumber {
            #[sql_type = "BigInt"]
            pub value: i64,
        }
        let next_seq = diesel::sql_query(sql.clone()).get_result::<SequenceNumber>(conn);
        log::info!("{}, {:?}", &sql, &next_seq);
        next_seq.unwrap_or_default().value
    }
    // ///For Hasura api
    // pub async fn get_hasura_schema(&self, hash: String) -> Result<impl Reply, Rejection> {
    //     if let Ok(conn) = self.get_connection() {
    //         let results = dsl::indexers
    //             .filter(dsl::hash.eq(hash.as_str()))
    //             .limit(1)
    //             .load::<Indexer>(conn.deref())
    //             .expect("Error loading indexers");
    //         if let (Some(indexer), Some(hasura_client)) =
    //             (results.get(0), self.hasura_client.as_ref())
    //         {
    //             if let Ok(value) = hasura_client.get_metadata(&indexer.namespace).await {
    //                 return Ok(warp::reply::json(&value));
    //             }
    //         }
    //     }
    //     Ok(warp::reply::json(&String::from("")))
    // }
}
