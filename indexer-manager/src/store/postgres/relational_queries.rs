///! This module contains the gory details of using Diesel to query
///! a database schema that is not known at compile time. The code in this
///! module is mostly concerned with constructing SQL queries and some
///! helpers for serializing and deserializing entities.
///!
///! Code in this module works very hard to minimize the number of allocations
///! that it performs
use diesel::pg::{Pg, PgConnection};
use diesel::query_builder::{AstPass, QueryFragment, QueryId};
use diesel::query_dsl::{LoadQuery, RunQueryDsl};
use diesel::result::{Error as DieselError, QueryResult};
use diesel::sql_types::{Array, Binary, Bool, Integer, Jsonb, Range, Text};
use diesel::Connection;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

use massbit::components::store::EntityType;
use massbit::data::store::scalar;
use massbit::prelude::*;

use crate::block_range::{
    BlockRange, BlockRangeContainsClause, BLOCK_RANGE_COLUMN, BLOCK_RANGE_CURRENT,
};
use crate::primary::Namespace;
use crate::relational::{Column, ColumnType, IdType, Layout, SqlName, Table, PRIMARY_KEY_COLUMN};
use crate::sql_value::SqlValue;

/// A string where we have checked that it is safe to embed it literally
/// in a string in a SQL query. In particular, we have escaped any use
/// of the string delimiter `'`.
///
/// This is only needed for `ParentIds::List` since we can't send those to
/// the database as a bind variable, and therefore need to embed them in
/// the query literally
#[derive(Debug, Clone)]
pub struct SafeString(String);

#[derive(Debug, Clone, Constructor)]
pub struct FindQuery<'a> {
    table: &'a Table,
    id: &'a str,
    block: BlockNumber,
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
    pub(crate) namespace: &'a Namespace,
    pub(crate) tables: Vec<&'a Table>,

    // Maps object name to ids.
    pub(crate) ids_for_type: BTreeMap<&'a EntityType, Vec<&'a str>>,
    pub(crate) block: BlockNumber,
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
            table
                .primary_key()
                .is_in(&self.ids_for_type[&table.object], &mut out)?;
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

#[derive(Debug)]
pub struct InsertQuery<'a> {
    table: &'a Table,
    entities: &'a [(EntityKey, Entity)],
    unique_columns: Vec<&'a Column>,
    block: BlockNumber,
}

impl<'a> InsertQuery<'a> {
    pub fn new(
        table: &'a Table,
        entities: &'a mut [(EntityKey, Entity)],
        block: BlockNumber,
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

/// Reduce the upper bound of the current entry's block range to `block` as
/// long as that does not result in an empty block range
#[derive(Debug, Clone, Constructor)]
pub struct ClampRangeQuery<'a, S> {
    table: &'a Table,
    entity_type: &'a EntityType,
    entity_ids: &'a [S],
    block: BlockNumber,
}

impl<'a, S> QueryFragment<Pg> for ClampRangeQuery<'a, S>
where
    S: AsRef<str> + diesel::serialize::ToSql<Text, Pg>,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        // update table
        //    set block_range = int4range(lower(block_range), $block)
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

/// Helper struct for returning the id's touched by the RevertRemove and
/// RevertExtend queries
#[derive(QueryableByName, PartialEq, Eq, Hash)]
pub struct ReturnedEntityData {
    #[sql_type = "Text"]
    pub id: String,
}

impl ReturnedEntityData {
    /// Convert primary key ids from Postgres' internal form to the format we
    /// use by stripping `\\x` off the front of bytes strings
    pub fn bytes_as_str(
        table: &Table,
        mut data: Vec<ReturnedEntityData>,
    ) -> Vec<ReturnedEntityData> {
        match table.primary_key().column_type.id_type() {
            IdType::String => data,
            IdType::Bytes => {
                for entry in data.iter_mut() {
                    entry.id = bytes_as_str(&entry.id);
                }
                data
            }
        }
    }
}

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

/// Conveniences for handling foreign keys depending on whether we are using
/// `IdType::Bytes` or `IdType::String` as the primary key
///
/// This trait adds some capabilities to `Column` that are very specific to
/// how we generate SQL queries. Using a method like `bind_ids` from this
/// trait on a given column means "send these values to the database in a form
/// that can later be used for comparisons with that column"
pub trait ForeignKeyClauses {
    /// The type of the column
    fn column_type(&self) -> &ColumnType;

    /// The name of the column
    fn name(&self) -> &str;

    /// Add `id` as a bind variable to `out`, using the right SQL type
    fn bind_id(&self, id: &str, out: &mut AstPass<Pg>) -> QueryResult<()> {
        match self.column_type().id_type() {
            IdType::String => out.push_bind_param::<Text, _>(&id)?,
            IdType::Bytes => out.push_bind_param::<Binary, _>(&str_as_bytes(&id)?.as_slice())?,
        }
        // Generate '::text' or '::bytea'
        out.push_sql("::");
        out.push_sql(self.column_type().sql_type());
        Ok(())
    }

    /// Add `ids`  as a bind variable to `out`, using the right SQL type
    fn bind_ids<S>(&self, ids: &[S], out: &mut AstPass<Pg>) -> QueryResult<()>
    where
        S: AsRef<str> + diesel::serialize::ToSql<Text, Pg>,
    {
        match self.column_type().id_type() {
            IdType::String => out.push_bind_param::<Array<Text>, _>(&ids)?,
            IdType::Bytes => {
                let ids = ids
                    .into_iter()
                    .map(|id| str_as_bytes(id.as_ref()))
                    .collect::<Result<Vec<scalar::Bytes>, _>>()?;
                let id_slices = ids.iter().map(|id| id.as_slice()).collect::<Vec<_>>();
                out.push_bind_param::<Array<Binary>, _>(&id_slices)?;
            }
        }
        // Generate '::text[]' or '::bytea[]'
        out.push_sql("::");
        out.push_sql(self.column_type().sql_type());
        out.push_sql("[]");
        Ok(())
    }

    /// Generate a clause `{name()} = $id` using the right types to bind `$id`
    /// into `out`
    fn eq(&self, id: &str, out: &mut AstPass<Pg>) -> QueryResult<()> {
        out.push_sql(self.name());
        out.push_sql(" = ");
        self.bind_id(id, out)
    }

    /// Generate a clause
    ///    `exists (select 1 from unnest($ids) as p(g$id) where id = p.g$id)`
    /// using the right types to bind `$ids` into `out`
    fn is_in<S>(&self, ids: &[S], out: &mut AstPass<Pg>) -> QueryResult<()>
    where
        S: AsRef<str> + diesel::serialize::ToSql<Text, Pg>,
    {
        out.push_sql("exists (select 1 from unnest(");
        self.bind_ids(ids, out)?;
        out.push_sql(") as p(g$id) where id = p.g$id)");
        Ok(())
    }

    /// Generate an array of arrays as literal SQL. The `ids` must form a
    /// valid matrix, i.e. the same numbe of entries in each row. This can
    /// be achieved by padding them with `None` values. Diesel does not support
    /// arrays of arrays as bind variables, nor arrays containing nulls, so
    /// we have to manually serialize the `ids` as literal SQL.
    fn push_matrix(
        &self,
        matrix: &Vec<Vec<Option<SafeString>>>,
        out: &mut AstPass<Pg>,
    ) -> QueryResult<()> {
        out.push_sql("array[");
        if matrix.is_empty() {
            // If there are no ids, make sure we are producing an
            // empty array of arrays
            out.push_sql("array[null]");
        } else {
            for (i, ids) in matrix.iter().enumerate() {
                if i > 0 {
                    out.push_sql(", ");
                }
                out.push_sql("array[");
                for (j, id) in ids.iter().enumerate() {
                    if j > 0 {
                        out.push_sql(", ");
                    }
                    match id {
                        None => out.push_sql("null"),
                        Some(id) => match self.column_type().id_type() {
                            IdType::String => {
                                out.push_sql("'");
                                out.push_sql(&id.0);
                                out.push_sql("'");
                            }
                            IdType::Bytes => {
                                out.push_sql("'\\x");
                                out.push_sql(&id.0.trim_start_matches("0x"));
                                out.push_sql("'");
                            }
                        },
                    }
                }
                out.push_sql("]");
            }
        }
        // Generate '::text[][]' or '::bytea[][]'
        out.push_sql("]::");
        out.push_sql(self.column_type().sql_type());
        out.push_sql("[][]");
        Ok(())
    }
}

impl ForeignKeyClauses for Column {
    fn column_type(&self) -> &ColumnType {
        &self.column_type
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }
}

pub trait FromEntityData: Default + From<Entity> {
    type Value: FromColumnValue;

    fn insert_entity_data(&mut self, key: String, v: Self::Value);
}

impl FromEntityData for Entity {
    type Value = massbit::prelude::Value;

    fn insert_entity_data(&mut self, key: String, v: Self::Value) {
        self.insert(key, v);
    }
}

impl FromEntityData for BTreeMap<String, q::Value> {
    type Value = q::Value;

    fn insert_entity_data(&mut self, key: String, v: Self::Value) {
        self.insert(key, v);
    }
}

pub trait FromColumnValue: Sized {
    fn is_null(&self) -> bool;

    fn null() -> Self;

    fn from_string(s: String) -> Self;

    fn from_bool(b: bool) -> Self;

    fn from_i32(i: i32) -> Self;

    fn from_big_decimal(d: scalar::BigDecimal) -> Self;

    fn from_big_int(i: serde_json::Number) -> Result<Self, StoreError>;

    // The string returned by the DB, without the leading '\x'
    fn from_bytes(i: &str) -> Result<Self, StoreError>;

    fn from_vec(v: Vec<Self>) -> Self;

    fn from_column_value(
        column_type: &ColumnType,
        json: serde_json::Value,
    ) -> Result<Self, StoreError> {
        use serde_json::Value as j;
        // Many possible conversion errors are already caught by how
        // we define the schema; for example, we can only get a NULL for
        // a column that is actually nullable
        match (json, column_type) {
            (j::Null, _) => Ok(Self::null()),
            (j::Bool(b), _) => Ok(Self::from_bool(b)),
            (j::Number(number), ColumnType::Int) => match number.as_i64() {
                Some(i) => i32::try_from(i).map(Self::from_i32).map_err(|e| {
                    StoreError::Unknown(anyhow!("failed to convert {} to Int: {}", number, e))
                }),
                None => Err(StoreError::Unknown(anyhow!(
                    "failed to convert {} to Int",
                    number
                ))),
            },
            (j::Number(number), ColumnType::BigDecimal) => {
                let s = number.to_string();
                scalar::BigDecimal::from_str(s.as_str())
                    .map(Self::from_big_decimal)
                    .map_err(|e| {
                        StoreError::Unknown(anyhow!(
                            "failed to convert {} to BigDecimal: {}",
                            number,
                            e
                        ))
                    })
            }
            (j::Number(number), ColumnType::BigInt) => Self::from_big_int(number),
            (j::Number(number), column_type) => Err(StoreError::Unknown(anyhow!(
                "can not convert number {} to {:?}",
                number,
                column_type
            ))),
            (j::String(s), ColumnType::String) | (j::String(s), ColumnType::Enum(_)) => {
                Ok(Self::from_string(s))
            }
            (j::String(s), ColumnType::Bytes) => Self::from_bytes(s.trim_start_matches("\\x")),
            (j::String(s), ColumnType::BytesId) => Ok(Self::from_string(bytes_as_str(&s))),
            (j::String(s), column_type) => Err(StoreError::Unknown(anyhow!(
                "can not convert string {} to {:?}",
                s,
                column_type
            ))),
            (j::Array(values), _) => Ok(Self::from_vec(
                values
                    .into_iter()
                    .map(|v| Self::from_column_value(column_type, v))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            (j::Object(_), _) => {
                unimplemented!("objects as entity attributes are not needed/supported")
            }
        }
    }
}

impl FromColumnValue for q::Value {
    fn is_null(&self) -> bool {
        self == &q::Value::Null
    }

    fn null() -> Self {
        Self::Null
    }

    fn from_string(s: String) -> Self {
        q::Value::String(s)
    }

    fn from_bool(b: bool) -> Self {
        q::Value::Boolean(b)
    }

    fn from_i32(i: i32) -> Self {
        q::Value::Int(i.into())
    }

    fn from_big_decimal(d: scalar::BigDecimal) -> Self {
        q::Value::String(d.to_string())
    }

    fn from_big_int(i: serde_json::Number) -> Result<Self, StoreError> {
        Ok(q::Value::String(i.to_string()))
    }

    fn from_bytes(b: &str) -> Result<Self, StoreError> {
        Ok(q::Value::String(format!("0x{}", b)))
    }

    fn from_vec(v: Vec<Self>) -> Self {
        q::Value::List(v)
    }
}

impl FromColumnValue for massbit::prelude::Value {
    fn is_null(&self) -> bool {
        self == &Value::Null
    }

    fn null() -> Self {
        Self::Null
    }

    fn from_string(s: String) -> Self {
        massbit::prelude::Value::String(s)
    }

    fn from_bool(b: bool) -> Self {
        massbit::prelude::Value::Bool(b)
    }

    fn from_i32(i: i32) -> Self {
        massbit::prelude::Value::Int(i)
    }

    fn from_big_decimal(d: scalar::BigDecimal) -> Self {
        massbit::prelude::Value::BigDecimal(d)
    }

    fn from_big_int(i: serde_json::Number) -> Result<Self, StoreError> {
        scalar::BigInt::from_str(&i.to_string())
            .map(massbit::prelude::Value::BigInt)
            .map_err(|e| StoreError::Unknown(anyhow!("failed to convert {} to BigInt: {}", i, e)))
    }

    fn from_bytes(b: &str) -> Result<Self, StoreError> {
        scalar::Bytes::from_str(b)
            .map(massbit::prelude::Value::Bytes)
            .map_err(|e| StoreError::Unknown(anyhow!("failed to convert {} to Bytes: {}", b, e)))
    }

    fn from_vec(v: Vec<Self>) -> Self {
        massbit::prelude::Value::List(v)
    }
}

/// Helper struct for retrieving entities from the database. With diesel, we
/// can only run queries that return columns whose number and type are known
/// at compile time. Because of that, we retrieve the actual data for an
/// entity as Jsonb by converting the row containing the entity using the
/// `to_jsonb` function.
#[derive(QueryableByName)]
pub struct EntityData {
    #[sql_type = "Text"]
    pub entity: String,
    #[sql_type = "Jsonb"]
    pub data: serde_json::Value,
}

impl EntityData {
    pub fn entity_type(&self) -> EntityType {
        EntityType::new(self.entity.clone())
    }

    /// Map the `EntityData` using the schema information in `Layout`
    pub fn deserialize_with_layout<T: FromEntityData>(
        self,
        layout: &Layout,
    ) -> Result<T, StoreError> {
        let entity_type = EntityType::new(self.entity);
        let table = layout.table_for_entity(&entity_type)?;

        use serde_json::Value as j;
        match self.data {
            j::Object(map) => {
                let mut out = T::default();
                out.insert_entity_data(
                    "__typename".to_owned(),
                    T::Value::from_string(entity_type.into_string()),
                );
                for (key, json) in map {
                    // Simply ignore keys that do not have an underlying table
                    // column; those will be things like the block_range that
                    // is used internally for versioning
                    if key == "g$parent_id" {
                        let value = T::Value::from_column_value(&ColumnType::String, json)?;
                        out.insert_entity_data("g$parent_id".to_owned(), value);
                    } else if let Some(column) = table.column(&SqlName::verbatim(key)) {
                        let value = T::Value::from_column_value(&column.column_type, json)?;
                        if !value.is_null() {
                            out.insert_entity_data(column.field.clone(), value);
                        }
                    }
                }
                Ok(out)
            }
            _ => unreachable!(
                "we use `to_json` in our queries, and will therefore always get an object back"
            ),
        }
    }
}

fn str_as_bytes(id: &str) -> QueryResult<scalar::Bytes> {
    scalar::Bytes::from_str(&id).map_err(|e| DieselError::SerializationError(Box::new(e)))
}

/// Convert Postgres string representation of bytes "\xdeadbeef"
/// to ours of just "deadbeef".
fn bytes_as_str(id: &str) -> String {
    id.trim_start_matches("\\x").to_owned()
}
