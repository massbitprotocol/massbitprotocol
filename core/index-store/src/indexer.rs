use crate::schema::indexers;
use massbit_common::prelude::diesel::prelude::*;
use crate::models::Indexer;
use strum_macros::AsStaticStr;
use strum::AsStaticRef;
use crate::establish_connection;
use massbit_common::prelude::diesel::result::Error;

// This is inspired by the syncing status from eth https://ethereum.stackexchange.com/questions/69458/sync-status-of-ethereum-node
#[derive(Clone, Debug, PartialEq, AsStaticStr)]
pub enum IndexerStatus {
    Synced,  // Meaning that the index is running
    Syncing, // This mean our index is not caught up to the latest block yet. We don't support this field yet
    False,   // Meaning that the index is not running
}
embed_migrations!("./migrations");
pub struct IndexerStore {

}
impl IndexerStore {
    pub fn create_indexer(hash: String, name: String, network: String, manifest: &Option<String>) {
        let id = format!("{}-{}", &name, &hash);
        let manifest_file = match manifest {
            None => String::from(""),
            Some(file) => file.clone()
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
            indexers::manifest.eq( &manifest_file)
            );
        if let Ok(indexer) = diesel::insert_into(indexers::table)
            .values(&values)
            .get_result::<Indexer>(&conn) {
            let namespace = format!("sgd{}", indexer.v_id);
            diesel::update(indexers::table.filter(indexers::v_id.eq(indexer.v_id)))
                .set(indexers::namespace.eq(&namespace))
                .execute(&conn);
        }
    }
    pub fn get_active_indexers() -> Vec<Indexer> {
        let conn = establish_connection();
        match embedded_migrations::run(&conn) {
            Ok(res) => println!("Finished embedded_migration {:?}", &res),
            Err(err) => println!("{:?}", &err)
        };
        match indexers::table.load::<Indexer>(&conn) {
            Ok(results) => results,
            Err(err) => {
                log::error!("Error while get indexer list: {:?}", &err);
                vec![]
            }
        }
    }
}
