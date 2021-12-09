use crate::model::{IndexerData, ListOptions};
use crate::orm::models::Indexer;
use crate::orm::schema::indexers;
use crate::orm::schema::indexers::dsl;
use crate::API_LIST_LIMIT;
use diesel::sql_types::BigInt;
use futures::lock::Mutex;
use log::debug;
//use massbit::components::link_resolver::LinkResolver as _;
use crate::{API_DEPLOY_ENDPOINT, API_ENDPOINT};
use massbit::ipfs_client::IpfsClient;
use massbit::ipfs_link_resolver::LinkResolver;
use massbit::prelude::prost::bytes::BufMut;
use massbit::prelude::{anyhow, TryStreamExt};
use massbit::slog::Logger;
use massbit_common::prelude::diesel::r2d2::ConnectionManager;
use massbit_common::prelude::diesel::{
    r2d2, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl,
};
use massbit_common::prelude::r2d2::PooledConnection;
use massbit_common::prelude::serde_json::{json, Value};
use std::ops::Deref;
use std::sync::Arc;
use tonic::body::empty_body;
use warp::{
    multipart::{FormData, Part},
    Rejection, Reply,
};

pub struct IndexerInfoService {
    pub connection_pool: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
    pub logger: Logger,
}

impl IndexerInfoService {
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
    /// for api deploy indexer from front-end
    pub async fn deploy_git_indexer(&self, content: IndexerData) -> Result<impl Reply, Rejection> {
        log::info!("Deploy new indexer from git {:?}.", &content);
        // Forward request to indexer API
        // let params = json!(
        //     {
        //         "repository": "https://github.com/massbitprotocol/serum_index.git",
        //         "name": "Serum indexer from github",
        //         "description": "Test git hub deploy api",
        //         "imageUrl": "https://staging.massbit.io/images/GIF/Keybanner-smol.gif"
        //     }
        // );

        let res = reqwest::Client::new()
            .post(&*API_DEPLOY_ENDPOINT)
            .json(&content)
            .send()
            .await;
        match res {
            Ok(res) => match res.json::<Value>().await {
                Ok(res) => {
                    println!("Forward deploying request successfully");
                    Ok(warp::reply::json(&res))
                }
                Err(e) => {
                    println!("Forward deploying request error response: {}", &e);
                    Ok(warp::reply::json(&json!({ "error": format!("{:?}", e) })))
                }
            },
            Err(e) => {
                println!("Forward deploying request error: {}", e);
                Ok(warp::reply::json(&json!({ "error": format!("{:?}", e) })))
            }
        }
    }
    /// for api deploy indexer from front-end
    pub async fn create_indexer(&self, content: IndexerData) -> Result<impl Reply, Rejection> {
        log::info!("Create new indexer from git {:?}.", &content);
        // Forward request to indexer API
        Ok(warp::reply::json(&content))
    }
    // async fn store_indexer(
    //     &self,
    //     manifest: &SolanaIndexerManifest,
    //     mut indexer: Indexer,
    // ) -> Result<Indexer, anyhow::Error> {
    //     indexer.got_block = -1_i64;
    //     if let Some(datasource) = manifest.data_sources.get(0) {
    //         indexer.address = datasource.source.address.clone();
    //         indexer.start_block = datasource.source.start_block.clone() as i64;
    //         indexer.network = datasource.network.clone();
    //         indexer.name = datasource.name.clone();
    //     }
    //     if IndexerRuntime::verify_manifest(manifest) {
    //         indexer.status = Some(String::from("Deploying"))
    //     } else {
    //         indexer.status = Some(String::from("Invalid"))
    //     }
    //     match self.get_connection() {
    //         Ok(conn) => {
    //             indexer.v_id = self.get_next_sequence(&conn, "indexers", "v_id");
    //             indexer.namespace = format!("sgd{:?}", &indexer.v_id);
    //             //let indexer = indexer.clone();
    //             diesel::insert_into(indexers::table)
    //                 .values(&indexer)
    //                 .get_result::<Indexer>(&conn)
    //                 .map_err(|err| anyhow!(format!("{:?}", &err)))
    //             //.expect("Error while create new indexer");
    //         }
    //         Err(err) => Err(anyhow!(format!("{:?}", &err))),
    //     }
    // }
    /// for api list indexer: /indexers?limit=?&offset=?
    pub async fn list_indexer(&self, options: ListOptions) -> Result<impl Reply, Rejection> {
        let content: Vec<Indexer> = vec![];
        if let Ok(conn) = self.get_connection() {
            match dsl::indexers
                .filter(dsl::deleted.eq(false))
                .order(dsl::v_id.asc())
                .offset(options.offset.unwrap_or_default())
                .limit(options.limit.unwrap_or(API_LIST_LIMIT))
                .load::<Indexer>(conn.deref())
            {
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
    /// for api get indexer detail: /indexers/:hash
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
}
