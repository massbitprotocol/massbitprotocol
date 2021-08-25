use crate::prelude::Arc;
//use crate::store::{Entity, EntityKey, StoreError};
//use graph::components::store::EntityType;

use crate::store::StoreError;
use graph::components::store::EntityType;
use graph::prelude::{BlockNumber, Entity, EntityKey, StopwatchMetrics};
use graph_store_postgres::command_support::Catalog;
use graph_store_postgres::layout_for_tests::Table;
use graph_store_postgres::relational_queries::{ClampRangeQuery, InsertQuery};
use massbit_common::prelude::diesel::{PgConnection, RunQueryDsl};
use std::collections::HashMap;

const POSTGRES_MAX_PARAMETERS: usize = u16::MAX as usize; // 65535
const DELETE_OPERATION_CHUNK_SIZE: usize = 1_000;

/// The size of string prefixes that we index. This is chosen so that we
/// will index strings that people will do string comparisons like
/// `=` or `!=` on; if text longer than this is stored in a String attribute
/// it is highly unlikely that they will be used for exact string operations.
/// This also makes sure that we do not put strings into a BTree index that's
/// bigger than Postgres' limit on such strings which is about 2k
pub const STRING_PREFIX_SIZE: usize = 256;
#[derive(Debug, Clone)]
pub struct Layout {
    /// Maps the GraphQL name of a type to the relational table
    pub tables: HashMap<EntityType, Arc<Table>>,
}
impl Layout {
    pub fn table_for_entity(&self, entity: &EntityType) -> Result<&Arc<Table>, StoreError> {
        self.tables
            .get(entity)
            .ok_or_else(|| StoreError::UnknownTable(entity.to_string()))
    }
    pub fn insert(
        &self,
        conn: &PgConnection,
        entity_type: &EntityType,
        entities: &mut [(EntityKey, Entity)],
        block: BlockNumber,
        stopwatch: &StopwatchMetrics,
    ) -> Result<usize, StoreError> {
        let table = self.table_for_entity(entity_type)?;
        let _section = stopwatch.start_section("insert_modification_insert_query");
        let mut count = 0;
        // Each operation must respect the maximum number of bindings allowed in PostgreSQL queries,
        // so we need to act in chunks whose size is defined by the number of entities times the
        // number of attributes each entity type has.
        // We add 1 to account for the `block_range` bind parameter
        let chunk_size = POSTGRES_MAX_PARAMETERS / (table.columns.len() + 1);
        for chunk in entities.chunks_mut(chunk_size) {
            count += InsertQuery::new(table, chunk, block)?
                .get_results(conn)
                .map(|ids| ids.len())?
        }
        Ok(count)
    }

    pub fn update(
        &self,
        conn: &PgConnection,
        entity_type: &EntityType,
        entities: &mut [(EntityKey, Entity)],
        block: BlockNumber,
        stopwatch: &StopwatchMetrics,
    ) -> Result<usize, StoreError> {
        let table = self.table_for_entity(&entity_type)?;
        let entity_keys: Vec<&str> = entities
            .iter()
            .map(|(key, _)| key.entity_id.as_str())
            .collect();

        let section = stopwatch.start_section("update_modification_clamp_range_query");
        ClampRangeQuery::new(table, &entity_type, &entity_keys, block).execute(conn)?;
        section.end();

        let _section = stopwatch.start_section("update_modification_insert_query");
        let mut count = 0;

        // Each operation must respect the maximum number of bindings allowed in PostgreSQL queries,
        // so we need to act in chunks whose size is defined by the number of entities times the
        // number of attributes each entity type has.
        // We add 1 to account for the `block_range` bind parameter
        let chunk_size = POSTGRES_MAX_PARAMETERS / (table.columns.len() + 1);
        for chunk in entities.chunks_mut(chunk_size) {
            count += InsertQuery::new(table, chunk, block)?.execute(conn)?;
        }
        Ok(count)
    }

    pub fn delete(
        &self,
        conn: &PgConnection,
        entity_type: &EntityType,
        entity_ids: &[String],
        block: BlockNumber,
        stopwatch: &StopwatchMetrics,
    ) -> Result<usize, StoreError> {
        let table = self.table_for_entity(&entity_type)?;
        let _section = stopwatch.start_section("delete_modification_clamp_range_query");
        let mut count = 0;
        for chunk in entity_ids.chunks(DELETE_OPERATION_CHUNK_SIZE) {
            count += ClampRangeQuery::new(table, &entity_type, chunk, block).execute(conn)?
        }
        Ok(count)
    }
}
