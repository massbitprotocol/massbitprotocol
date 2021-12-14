use crate::manager::{IndexerManager, IndexerRuntime};
use crate::model::{IndexerData, ListOptions};
use crate::orm::schema::indexers;
use crate::orm::schema::indexers::dsl;
use crate::orm::IndexerStatus;
use crate::API_LIST_LIMIT;
use chain_solana::SolanaIndexerManifest;
use diesel::sql_types::BigInt;
use futures::lock::Mutex;
use log::debug;
//use massbit::components::link_resolver::LinkResolver as _;
use crate::orm::models::Indexer;
use crate::FILES;
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
use massbit_common::prelude::serde_json::json;
use solana_sdk::stake::instruction::StakeInstruction::Deactivate;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
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
                .filter(dsl::status.ne(IndexerStatus::Draft))
                .load::<Indexer>(conn.deref())
                .ok()
        })
    }

    /// for api deploy indexer from front-end
    pub async fn deploy_indexer_request(
        &self,
        content: Indexer,
        indexer_manager: Arc<Mutex<IndexerManager>>,
    ) -> Result<impl Reply, Rejection> {
        log::info!("Deploy new indexer from git {:?}.", &content);
        let mut indexer = content;
        let mut manifest: Option<SolanaIndexerManifest> = None;

        //if let Ok(map) = git_helper.load_indexer().await {
        let mut map = HashMap::new();
        map.insert(
            "mapping",
            self.ipfs_client
                .cat_all(&indexer.mapping, None)
                .await
                .unwrap(),
        );
        map.insert(
            "manifest",
            self.ipfs_client
                .cat_all(&indexer.manifest, None)
                .await
                .unwrap(),
        );
        map.insert(
            "graphql",
            self.ipfs_client
                .cat_all(&indexer.graphql, None)
                .await
                .unwrap(),
        );

        log::info!(
            "Finished load indexer from git, content {:#?}.",
            &map.keys()
        );

        for (file_name, file_content) in map {
            let values = file_content.to_vec();
            // Return manifest content for parser
            if file_name == "manifest" {
                let link_resolver = LinkResolver::from(self.ipfs_client.clone());
                manifest = IndexerRuntime::parse_manifest(
                    &indexer.hash,
                    &values,
                    link_resolver,
                    &self.logger,
                )
                .await
                .ok();
            }
        }

        let hash = indexer.hash.clone();
        if let Some(manifest) = &manifest {
            match self.update_indexer(manifest, indexer).await {
                Err(err) => {
                    log::error!("{:?}", &err);
                    return Ok(warp::reply::json(&json!({ "error": &err.to_string() })));
                }
                Ok(indexer) => {
                    if let Err(err) = indexer_manager
                        .lock()
                        .await
                        .start_indexer(indexer.clone())
                        .await
                    {
                        log::error!("{:?}", &err);
                        return Ok(warp::reply::json(&json!({ "error": &err.to_string() })));
                    } else {
                        return Ok(warp::reply::json(&json!({ "id": hash })));
                    }
                }
            };
        };
        return Ok(warp::reply::json(&json!({ "error": "Cannot deploy" })));
    }
    async fn update_indexer(
        &self,
        manifest: &SolanaIndexerManifest,
        mut indexer: Indexer,
    ) -> Result<Indexer, anyhow::Error> {
        indexer.got_block = -1_i64;
        if let Some(datasource) = manifest.data_sources.get(0) {
            indexer.address = datasource.source.address.clone();
            indexer.start_block = datasource.source.start_block.clone() as i64;
            indexer.network = datasource.network.clone();
            // If the name from deploy request exist, use it. If it is not exist, use datasource.name
            if indexer.name.is_empty() {
                indexer.name = datasource.name.clone();
            }
        }
        if IndexerRuntime::verify_manifest(manifest) {
            indexer.status = IndexerStatus::Deploying
        } else {
            indexer.status = IndexerStatus::Invalid
        }
        match self.get_connection() {
            Ok(conn) => {
                //indexer.v_id = self.get_next_sequence(&conn, "indexers", "v_id");
                indexer.namespace = format!("sgd{:?}", &indexer.v_id);
                diesel::update(dsl::indexers.filter(dsl::hash.eq(&indexer.hash)))
                    .set((
                        dsl::got_block.eq(&indexer.got_block),
                        dsl::name.eq(&indexer.name),
                        dsl::network.eq(&indexer.network),
                        dsl::address.eq(&indexer.address),
                        dsl::status.eq(IndexerStatus::Deployed),
                    ))
                    .execute(conn.deref());
                Ok(indexer)
            }
            Err(err) => Err(anyhow!(format!("{:?}", &err))),
        }
    }

    // async fn update_indexer(&self, mut indexer: &Indexer) -> Result<(), anyhow::Error> {
    //     match self.get_connection() {
    //         Ok(conn) => {
    //             if let Err(err) = diesel::update(dsl::indexers.filter(dsl::hash.eq(&indexer.hash)))
    //                 .set((
    //                     dsl::namespace.eq(&indexer.namespace),
    //                     dsl::manifest.eq(&indexer.manifest),
    //                     dsl::mapping.eq(&indexer.mapping),
    //                     dsl::graphql.eq(&indexer.graphql),
    //                     dsl::address.eq(&indexer.address),
    //                     dsl::start_block.eq(&indexer.start_block),
    //                     dsl::network.eq(&indexer.network),
    //                     dsl::version.eq(&indexer.version),
    //                     dsl::status.eq(&indexer.status),
    //                 ))
    //                 .execute(conn.deref())
    //             {
    //                 log::error!("{:?}", &err);
    //                 return Err(anyhow!(format!("{:?}", &err)));
    //             }
    //             Ok(())
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
