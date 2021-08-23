use crate::hasura::track_hasura_with_ddl_gen_plugin;
use crate::type_index::{IndexConfig, IndexStore};

pub async fn run_ddl_gen(index_config: &IndexConfig) {
    // Create tables for the new index
    IndexStore::migrate_with_ddl_gen_plugin(
        &index_config.identifier.name_with_hash,
        &index_config.schema,
        &index_config.config,
    );

    // Track the newly created tables in hasura
    track_hasura_with_ddl_gen_plugin(&index_config.identifier.name_with_hash).await;
}
