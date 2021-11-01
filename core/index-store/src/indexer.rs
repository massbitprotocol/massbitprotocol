use crate::establish_connection;
use crate::models::Indexer;
use crate::schema::indexers;
use massbit_common::prelude::diesel::prelude::*;

use strum::AsStaticRef;
use strum_macros::AsStaticStr;

// This is inspired by the syncing status from eth https://ethereum.stackexchange.com/questions/69458/sync-status-of-ethereum-node
#[derive(Clone, Debug, PartialEq, AsStaticStr)]
pub enum IndexerStatus {
    Synced,  // Meaning that the index is running
    Syncing, // This mean our index is not caught up to the latest block yet. We don't support this field yet
    False,   // Meaning that the index is not running
}
embed_migrations!("./migrations");
//Todo: Improve this index store: use connection pool instead of single connection
pub struct IndexerStore {}
impl IndexerStore {
    pub fn run_migration() {
        let conn = establish_connection();
        match embedded_migrations::run_with_output(&conn, &mut std::io::stdout()) {
            Ok(res) => log::info!("Finished embedded_migrations {:?}", &res),
            Err(err) => log::error!("{:?}", &err),
        };
    }
    pub fn create_indexer(hash: String, name: String, network: String, manifest: &Option<String>) {
        let id = format!("{}-{}", &name, &hash);
        let manifest_file = match manifest {
            None => String::from(""),
            Some(file) => file.clone(),
        };
        let conn = establish_connection();
        let values = (
            indexers::id.eq(&id),
            indexers::network.eq(&network),
            indexers::name.eq(&name),
            indexers::namespace.eq(""),
            indexers::description.eq(""),
            indexers::repo.eq(""),
            indexers::index_status.eq(IndexerStatus::Synced.as_static().to_lowercase()),
            indexers::got_block.eq(0),
            indexers::hash.eq(&hash),
            indexers::manifest.eq(&manifest_file),
        );
        if let Ok(indexer) = diesel::insert_into(indexers::table)
            .values(&values)
            .get_result::<Indexer>(&conn)
        {
            let namespace = format!("sgd{}", indexer.v_id);
            diesel::update(indexers::table.filter(indexers::v_id.eq(indexer.v_id)))
                .set(indexers::namespace.eq(&namespace))
                .execute(&conn);
        }
    }
    pub fn get_active_indexers() -> Vec<Indexer> {
        let conn = establish_connection();
        match indexers::table.load::<Indexer>(&conn) {
            Ok(results) => results,
            Err(err) => {
                log::error!("Error while get indexer list: {:?}", &err);
                vec![]
            }
        }
    }
    pub fn store_got_block(hash: &String, got_block_slot: i64) {
        let conn = establish_connection();
        diesel::update(indexers::table)
            .filter(indexers::id.eq(hash))
            .set(indexers::got_block.eq(got_block_slot))
            .execute(&conn);
    }
}
