pub mod advisory_lock;
pub mod catalog;
pub mod connection_pool;
pub mod store_builder;
use crate::store::{
    Entity, EntityKey, EntityModification, EntityType, QueryExecutionError, StoreError,
    WritableStore,
};
pub use connection_pool::ConnectionPool;
use index_store::core::Store;
use massbit_common::prelude::structmap;
use std::collections::{BTreeMap, HashMap};
#[derive(Clone)]
pub struct PostgresIndexStore {
    pub connection_string: String,
    //pub connection_pool: ConnectionPool,
    //buffer: HashMap<String, TableBuffer>,
    //entity_dependencies: HashMap<String, Vec<String>>,
}

impl PostgresIndexStore {
    pub async fn new(connection_str: &str) -> PostgresIndexStore {
        PostgresIndexStore {
            connection_string: String::from(connection_str),
        }
    }
}

impl Store for PostgresIndexStore {
    fn save(&mut self, entity_name: String, data: structmap::GenericMap) {
        todo!()
    }

    fn flush(&mut self) {
        todo!()
    }
}
impl WritableStore for PostgresIndexStore {
    fn get(&self, key: EntityKey) -> Result<Option<Entity>, QueryExecutionError> {
        let mut entity = Entity::new();
        let id = key.entity_id.as_str();
        entity.set("id", key.entity_id.as_str());
        match id {
            "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32" => {
                entity.set("pairCount", 0);
                Ok(Some(entity))
            }
            _ => Ok(None),
        }
    }

    fn get_many(
        &self,
        ids_for_type: BTreeMap<&EntityType, Vec<&str>>,
    ) -> Result<BTreeMap<EntityType, Vec<Entity>>, StoreError> {
        println!("{:?}", ids_for_type);
        let result: BTreeMap<EntityType, Vec<Entity>> = BTreeMap::default();
        Ok(result)
    }

    fn transact_block_operations(&self, mods: Vec<EntityModification>) -> Result<(), StoreError> {
        mods.iter()
            .for_each(|modification| println!("{:?}", modification));
        //let conn = self.get_conn()?;
        /*
        let event = conn.transaction(|| -> Result<_, StoreError> {
            // Emit a store event for the changes we are about to make. We
            // wait with sending it until we have done all our other work
            // so that we do not hold a lock on the notification queue
            // for longer than we have to
            let event: StoreEvent = mods.iter().collect();

            // Make the changes
            let layout = self.layout(&conn, site.clone())?;
            let section = stopwatch.start_section("apply_entity_modifications");
            let count = self.apply_entity_modifications(
                &conn,
                layout.as_ref(),
                mods,
                &block_ptr_to,
                stopwatch,
            )?;
            deployment::update_entity_count(
                &conn,
                site.as_ref(),
                layout.count_query.as_str(),
                count,
            )?;
            section.end();

            dynds::insert(&conn, &site.deployment, data_sources, &block_ptr_to)?;

            if !deterministic_errors.is_empty() {
                deployment::insert_subgraph_errors(
                    &conn,
                    &site.deployment,
                    deterministic_errors,
                    block_ptr_to.block_number(),
                )?;
            }

            deployment::forward_block_ptr(&conn, &site.deployment, block_ptr_to)?;
            Ok(event)
        })?;
         */
        Ok(())
    }
}
