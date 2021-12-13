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
    /// for api deploy indexer from Indexer-API
    pub async fn deploy_indexer(
        &self,
        form: FormData,
        indexer_manager: Arc<Mutex<IndexerManager>>,
    ) -> Result<impl Reply, Rejection> {
        log::info!("Deploy new indexer");
        let parts: Vec<Part> = form.try_collect().await.map_err(|e| {
            eprintln!("form error: {}", e);
            warp::reject::reject()
        })?;
        let mut indexer = Indexer::new();
        let mut manifest: Option<SolanaIndexerManifest> = None;
        for p in parts {
            log::info!("Receive file: {}/{}", &p.name(), p.filename().unwrap());
            let name = format!("{}", &p.name());
            let p_name = name.as_str();
            match p_name {
                "mapping" | "schema" | "manifest" => {
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
                    if p_name == "manifest" {
                        let link_resolver = LinkResolver::from(self.ipfs_client.clone());
                        manifest = IndexerRuntime::parse_manifest(
                            &indexer.hash,
                            &value,
                            link_resolver,
                            &self.logger,
                        )
                        .await
                        .ok();
                    }
                    match self.ipfs_client.add(value).await {
                        Ok(response) => match p_name {
                            "mapping" => indexer.mapping = response.hash.clone(),
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
        if let Some(manifest) = &manifest {
            if let Ok(indexer) = self.store_indexer(manifest, indexer).await {
                if let Err(err) = indexer_manager.lock().await.start_indexer(indexer).await {
                    log::error!("{:?}", &err);
                };
            }
        };
        Ok("success")
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

        for (file_name, content) in map {
            let values = content.to_vec();
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

        debug!("indexer: {:?}", &indexer);
        let hash = indexer.hash.clone();
        if let Some(manifest) = &manifest {
            if let Ok(indexer) = self.store_indexer(manifest, indexer).await {
                if let Err(err) = indexer_manager.lock().await.start_indexer(indexer).await {
                    log::error!("{:?}", &err);
                };
            }
        };
        Ok(warp::reply::json(&json!({ "id": hash })))
    }
    async fn store_indexer(
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
                // let indexer = indexer.clone();
                // diesel::insert_into(indexers::table)
                //     .values(&indexer)
                //     .get_result::<Indexer>(&conn)
                //     .map_err(|err| anyhow!(format!("{:?}", &err)))
                // .expect("Error while create new indexer");
            }
            Err(err) => Err(anyhow!(format!("{:?}", &err))),
        }
    }
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
