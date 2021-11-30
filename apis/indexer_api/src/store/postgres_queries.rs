use crate::store::block_range::BlockRange;
use crate::store::entity_data::EntityData;
use crate::store::sql_value::SqlValue;
use chain_solana::types::BlockSlot;
use derive_more::Constructor;
use diesel::pg::{Pg, PgConnection};
use diesel::query_builder::{AstPass, QueryFragment, QueryId};
use diesel::query_dsl::LoadQuery;
use diesel::result::{Error as DieselError, QueryResult};
use massbit::components::store::StoreError;
use massbit::prelude::CheapClone;
use massbit_common::prelude::diesel::sql_types::{
    Array, BigInt, Binary, Bool, Integer, Range, Text,
};
use massbit_common::prelude::diesel::{Connection, RunQueryDsl};
use massbit_solana_sdk::entity::{Entity, Value};
use massbit_solana_sdk::model::{EntityKey, BLOCK_NUMBER_MAX};
use massbit_solana_sdk::scalar;
use massbit_store_postgres::relational::{Column, ColumnType, Table};
use massbit_store_postgres::relational_queries::{ForeignKeyClauses, ReturnedEntityData};
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

pub const POSTGRES_MAX_PARAMETERS: usize = u16::MAX as usize; // 65535
pub const DELETE_OPERATION_CHUNK_SIZE: usize = 1_000;
/// The name of the column in which we store the block range
pub const BLOCK_RANGE_COLUMN: &str = "block_range";
pub const PRIMARY_KEY_COLUMN: &str = "id";
/// The SQL clause we use to check that an entity version is current;
/// that version has an unbounded block range, but checking for
/// `upper_inf(block_range)` is slow and can't use the exclusion
/// index we have on entity tables; we therefore check if i32::MAX is
/// in the range
pub const BLOCK_RANGE_CURRENT: &str = "block_range @> 2147483647";

/// A `QueryValue` makes it possible to bind a `Value` into a SQL query
/// using the metadata from Column
struct QueryValue<'a>(&'a Value, &'a ColumnType);

impl<'a> QueryFragment<Pg> for QueryValue<'a> {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        let column_type = self.1;

        match self.0 {
            Value::String(s) => match &column_type {
                ColumnType::String => out.push_bind_param::<Text, _>(s),
                ColumnType::Enum(enum_type) => {
                    out.push_bind_param::<Text, _>(s)?;
                    out.push_sql("::");
                    out.push_sql(enum_type.name.as_str());
                    Ok(())
                }
                ColumnType::Bytes | ColumnType::BytesId => {
                    let bytes = scalar::Bytes::from_str(&s)
                        .map_err(|e| DieselError::SerializationError(Box::new(e)))?;
                    out.push_bind_param::<Binary, _>(&bytes.as_slice())
                }
                _ => unreachable!(
                    "only string, enum and tsvector columns have values of type string"
                ),
            },
            Value::Int(i) => out.push_bind_param::<Integer, _>(i),
            Value::BigDecimal(d) => {
                out.push_bind_param::<Text, _>(&d.to_string())?;
                out.push_sql("::numeric");
                Ok(())
            }
            Value::Bool(b) => out.push_bind_param::<Bool, _>(b),
            Value::List(values) => {
                let sql_values = SqlValue::new_array(values.clone());
                match &column_type {
                    ColumnType::BigDecimal | ColumnType::BigInt => {
                        let text_values: Vec<_> = values.iter().map(|v| v.to_string()).collect();
                        out.push_bind_param::<Array<Text>, _>(&text_values)?;
                        out.push_sql("::numeric[]");
                        Ok(())
                    }
                    ColumnType::Boolean => out.push_bind_param::<Array<Bool>, _>(&sql_values),
                    ColumnType::Bytes => out.push_bind_param::<Array<Binary>, _>(&sql_values),
                    ColumnType::Int => out.push_bind_param::<Array<Integer>, _>(&sql_values),
                    ColumnType::String => out.push_bind_param::<Array<Text>, _>(&sql_values),
                    ColumnType::Enum(enum_type) => {
                        out.push_bind_param::<Array<Text>, _>(&sql_values)?;
                        out.push_sql("::");
                        out.push_sql(enum_type.name.as_str());
                        out.push_sql("[]");
                        Ok(())
                    }
                    ColumnType::BytesId => out.push_bind_param::<Array<Binary>, _>(&sql_values),
                }
            }
            Value::Null => {
                out.push_sql("null");
                Ok(())
            }
            Value::Bytes(b) => out.push_bind_param::<Binary, _>(&b.as_slice()),
            Value::BigInt(i) => {
                out.push_bind_param::<Text, _>(&i.to_string())?;
                out.push_sql("::numeric");
                Ok(())
            }
        }
    }
}

#[derive(Debug)]
pub struct InsertQuery<'a> {
    table: &'a Table,
    entities: &'a [(EntityKey, Entity)],
    unique_columns: Vec<&'a Column>,
    block: BlockSlot,
}

impl<'a> InsertQuery<'a> {
    pub fn new(
        table: &'a Table,
        entities: &'a mut [(EntityKey, Entity)],
        block: BlockSlot,
    ) -> Result<InsertQuery<'a>, StoreError> {
        for (entity_key, entity) in entities.iter_mut() {
            for column in table.columns.iter() {
                if !column.is_nullable() && !entity.contains_key(&column.field) {
                    return Err(StoreError::QueryExecutionError(format!(
                        "can not insert entity {}[{}] since value for non-nullable attribute {} is missing. \
                     To fix this, mark the attribute as nullable in the GraphQL schema or change the \
                     mapping code to always set this attribute.",
                        entity_key.entity_type, entity_key.entity_id, column.field
                    )));
                }
            }
        }
        let unique_columns = InsertQuery::unique_columns(table, entities);

        Ok(InsertQuery {
            table,
            entities,
            unique_columns,
            block,
        })
    }

    /// Build the column name list using the subset of all keys among present entities.
    fn unique_columns(table: &'a Table, entities: &'a [(EntityKey, Entity)]) -> Vec<&'a Column> {
        let mut hashmap = HashMap::new();
        for (_key, entity) in entities.iter() {
            for column in &table.columns {
                if entity.get(&column.field).is_some() {
                    hashmap.entry(column.name.as_str()).or_insert(column);
                }
            }
        }
        hashmap.into_iter().map(|(_key, value)| value).collect()
    }
}

impl<'a> QueryFragment<Pg> for InsertQuery<'a> {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();

        // Construct a query
        //   insert into schema.table(column, ...)
        //   values
        //   (a, b, c),
        //   (d, e, f)
        //   [...]
        //   (x, y, z)
        //
        // and convert and bind the entity's values into it
        out.push_sql("insert into ");
        out.push_sql(self.table.qualified_name.as_str());

        out.push_sql("(");

        for &column in &self.unique_columns {
            out.push_identifier(column.name.as_str())?;
            out.push_sql(", ");
        }
        out.push_identifier(BLOCK_RANGE_COLUMN)?;

        out.push_sql(") values\n");

        // Use a `Peekable` iterator to help us decide how to finalize each line.
        let mut iter = self.entities.iter().map(|(_key, entity)| entity).peekable();
        while let Some(entity) = iter.next() {
            out.push_sql("(");
            for column in &self.unique_columns {
                // If the column name is not within this entity's fields, we will issue the
                // null value in its place
                if let Some(value) = entity.get(&column.field) {
                    QueryValue(value, &column.column_type).walk_ast(out.reborrow())?;
                } else {
                    out.push_sql("null");
                }
                out.push_sql(", ");
            }
            let block_range: BlockRange = (self.block..).into();
            out.push_bind_param::<Range<Integer>, _>(&block_range)?;
            out.push_sql(")");

            // finalize line according to remaining entities to insert
            if iter.peek().is_some() {
                out.push_sql(",\n");
            }
        }
        out.push_sql("\nreturning ");
        out.push_sql(PRIMARY_KEY_COLUMN);
        out.push_sql("::text");

        Ok(())
    }
}

impl<'a> QueryId for InsertQuery<'a> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<'a> LoadQuery<PgConnection, ReturnedEntityData> for InsertQuery<'a> {
    fn internal_load(self, conn: &PgConnection) -> QueryResult<Vec<ReturnedEntityData>> {
        conn.query_by_name(&self)
            .map(|data| ReturnedEntityData::bytes_as_str(&self.table, data))
    }
}

impl<'a, Conn> RunQueryDsl<Conn> for InsertQuery<'a> {}

#[derive(Debug, Clone, Constructor)]
pub struct FindQuery<'a> {
    table: &'a Table,
    id: &'a str,
    block: BlockSlot,
}

impl<'a> QueryFragment<Pg> for FindQuery<'a> {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();

        // Generate
        //    select '..' as entity, to_jsonb(e.*) as data
        //      from schema.table e where id = $1
        out.push_sql("select ");
        out.push_bind_param::<Text, _>(&self.table.object.as_str())?;
        out.push_sql(" as entity, to_jsonb(e.*) as data\n");
        out.push_sql("  from ");
        out.push_sql(self.table.qualified_name.as_str());
        out.push_sql(" e\n where ");
        self.table.primary_key().eq(&self.id, &mut out)?;
        out.push_sql(" and ");
        BlockRangeContainsClause::new(&self.table, "e.", self.block).walk_ast(out)
    }
}

impl<'a> QueryId for FindQuery<'a> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<'a> LoadQuery<PgConnection, EntityData> for FindQuery<'a> {
    fn internal_load(self, conn: &PgConnection) -> QueryResult<Vec<EntityData>> {
        conn.query_by_name(&self)
    }
}

impl<'a, Conn> RunQueryDsl<Conn> for FindQuery<'a> {}

#[derive(Debug, Clone, Constructor)]
pub struct FindManyQuery<'a> {
    pub(crate) tables: Vec<&'a Table>,

    // Maps object name to ids.
    pub(crate) ids_for_type: BTreeMap<&'a String, Vec<&'a str>>,
    pub(crate) block: BlockSlot,
}

impl<'a> QueryFragment<Pg> for FindManyQuery<'a> {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();

        // Generate
        //    select $object0 as entity, to_jsonb(e.*) as data
        //      from schema.<table0> e where {id.is_in($ids0)}
        //    union all
        //    select $object1 as entity, to_jsonb(e.*) as data
        //      from schema.<table1> e where {id.is_in($ids1))
        //    union all
        //    ...
        for (i, table) in self.tables.iter().enumerate() {
            if i > 0 {
                out.push_sql("\nunion all\n");
            }
            out.push_sql("select ");
            out.push_bind_param::<Text, _>(&table.object.as_str())?;
            out.push_sql(" as entity, to_jsonb(e.*) as data\n");
            out.push_sql("  from ");
            out.push_sql(table.qualified_name.as_str());
            out.push_sql(" e\n where ");
            let entity_name = table.object.cheap_clone().into_string();
            table
                .primary_key()
                .is_in(&self.ids_for_type[&entity_name], &mut out)?;
            out.push_sql(" and ");
            BlockRangeContainsClause::new(&table, "e.", self.block).walk_ast(out.reborrow())?;
        }
        Ok(())
    }
}

impl<'a> QueryId for FindManyQuery<'a> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<'a> LoadQuery<PgConnection, EntityData> for FindManyQuery<'a> {
    fn internal_load(self, conn: &PgConnection) -> QueryResult<Vec<EntityData>> {
        conn.query_by_name(&self)
    }
}

impl<'a, Conn> RunQueryDsl<Conn> for FindManyQuery<'a> {}

/// Generate the clause that checks whether `block` is in the block range
/// of an entity
#[derive(Constructor)]
pub struct BlockRangeContainsClause<'a> {
    table: &'a Table,
    table_prefix: &'a str,
    block: BlockSlot,
}

impl<'a> QueryFragment<Pg> for BlockRangeContainsClause<'a> {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();

        out.push_sql(self.table_prefix);
        out.push_identifier(BLOCK_RANGE_COLUMN)?;
        out.push_sql(" @> ");
        out.push_bind_param::<Integer, _>(&self.block)?;
        if self.table.is_account_like && self.block < BLOCK_NUMBER_MAX {
            // When block is BLOCK_NUMBER_MAX, these checks would be wrong; we
            // don't worry about adding the equivalent in that case since
            // we generally only see BLOCK_NUMBER_MAX here for metadata
            // queries where block ranges don't matter anyway
            out.push_sql(" and coalesce(upper(");
            out.push_identifier(BLOCK_RANGE_COLUMN)?;
            out.push_sql("), 2147483647) > ");
            out.push_bind_param::<Integer, _>(&self.block)?;
            out.push_sql(" and lower(");
            out.push_identifier(BLOCK_RANGE_COLUMN)?;
            out.push_sql(") <= ");
            out.push_bind_param::<Integer, _>(&self.block)
        } else {
            Ok(())
        }
    }
}

/// Reduce the upper bound of the current entry's block range to `block` as
/// long as that does not result in an empty block range
#[derive(Debug, Clone, Constructor)]
pub struct ClampRangeQuery<'a, S> {
    table: &'a Table,
    entity_type: &'a String,
    entity_ids: &'a [S],
    block: BlockSlot,
}

impl<'a, S> QueryFragment<Pg> for ClampRangeQuery<'a, S>
where
    S: AsRef<str> + diesel::serialize::ToSql<Text, Pg>,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        // update table
        //    set block_range = int4rangee(lower(block_range), $block)
        //  where id in (id1, id2, ..., idN)
        //    and block_range @> INTMAX
        out.unsafe_to_cache_prepared();
        out.push_sql("update ");
        out.push_sql(self.table.qualified_name.as_str());
        out.push_sql("\n   set ");
        out.push_identifier(BLOCK_RANGE_COLUMN)?;
        out.push_sql(" = int4range(lower(");
        out.push_identifier(BLOCK_RANGE_COLUMN)?;
        out.push_sql("), ");
        out.push_bind_param::<Integer, _>(&self.block)?;
        out.push_sql(")\n where ");

        self.table.primary_key().is_in(self.entity_ids, &mut out)?;
        out.push_sql(" and (");
        out.push_sql(BLOCK_RANGE_CURRENT);
        out.push_sql(")");

        Ok(())
    }
}

impl<'a, S> QueryId for ClampRangeQuery<'a, S>
where
    S: AsRef<str> + diesel::serialize::ToSql<Text, Pg>,
{
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<'a, S, Conn> RunQueryDsl<Conn> for ClampRangeQuery<'a, S> {}
