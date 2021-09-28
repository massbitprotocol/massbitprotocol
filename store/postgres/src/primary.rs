//! Utilities for dealing with indexer metadata that resides in the primary
//! shard. Anything in this module can only be used with a database connection
//! for the primary shard.
use diesel::{
    data_types::PgTimestamp,
    dsl::{any, exists, not, select},
    pg::Pg,
    serialize::Output,
    sql_types::{Array, Integer, Text},
    types::{FromSql, ToSql},
};
use diesel::{
    dsl::{delete, insert_into, sql, update},
    r2d2::PooledConnection,
};
use diesel::{pg::PgConnection, r2d2::ConnectionManager};
use diesel::{
    prelude::{
        BoolExpressionMethods, ExpressionMethods, GroupByDsl, JoinOnDsl, NullableExpressionMethods,
        OptionalExtension, QueryDsl, RunQueryDsl,
    },
    Connection as _,
};
use massbit::components::store::DeploymentId as IndexerDeploymentId;
use massbit::data::schema::generate_entity_id;
use massbit::prelude::*;
use maybe_owned::MaybeOwned;
use std::{
    collections::HashMap,
    convert::TryFrom,
    convert::TryInto,
    fmt,
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::block_range::UNVERSIONED_RANGE;
use crate::Shard;

table! {
    indexer (vid) {
        vid -> BigInt,
        id -> Text,
        name -> Text,
        current_version -> Nullable<Text>,
        pending_version -> Nullable<Text>,
        created_at -> Numeric,
        block_range -> Range<Integer>,
    }
}

table! {
    deployment_schemas(id) {
        id -> Integer,
        created_at -> Timestamptz,
        indexer -> Text,
        name -> Text,
        shard -> Text,
        network -> Text,
        /// If there are multiple entries for the same IPFS hash (`subgraph`)
        /// only one of them will be active. That's the one we use for
        /// querying
        active -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(indexer, deployment_schemas,);

#[derive(Clone, Debug, PartialEq, Eq, Hash, AsExpression, FromSqlRow)]
#[sql_type = "diesel::sql_types::Text"]
/// A namespace (schema) in the database
pub struct Namespace(String);

impl Namespace {
    pub fn new(s: String) -> Result<Self, String> {
        // Normal database namespaces must be of the form `sgd[0-9]+`
        if !s.starts_with("sgd") || s.len() <= 3 {
            return Err(s);
        }
        for c in s.chars().skip(3) {
            if !c.is_numeric() {
                return Err(s);
            }
        }

        Ok(Namespace(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl FromSql<Text, Pg> for Namespace {
    fn from_sql(bytes: Option<&[u8]>) -> diesel::deserialize::Result<Self> {
        let s = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        Namespace::new(s).map_err(Into::into)
    }
}

impl ToSql<Text, Pg> for Namespace {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> diesel::serialize::Result {
        <String as ToSql<Text, Pg>>::to_sql(&self.0, out)
    }
}

/// A marker that an `i32` references a deployment. Values of this type hold
/// the primary key from the `deployment_schemas` table
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, AsExpression, FromSqlRow)]
#[sql_type = "diesel::sql_types::Integer"]
pub struct DeploymentId(i32);

impl fmt::Display for DeploymentId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl From<DeploymentId> for IndexerDeploymentId {
    fn from(id: DeploymentId) -> Self {
        IndexerDeploymentId::new(id.0)
    }
}

impl From<IndexerDeploymentId> for DeploymentId {
    fn from(id: IndexerDeploymentId) -> Self {
        DeploymentId(id.0)
    }
}

impl FromSql<Integer, Pg> for DeploymentId {
    fn from_sql(bytes: Option<&[u8]>) -> diesel::deserialize::Result<Self> {
        let id = <i32 as FromSql<Integer, Pg>>::from_sql(bytes)?;
        Ok(DeploymentId(id))
    }
}

impl ToSql<Integer, Pg> for DeploymentId {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> diesel::serialize::Result {
        <i32 as ToSql<Integer, Pg>>::to_sql(&self.0, out)
    }
}

#[derive(Debug)]
/// Details about a deployment and the shard in which it is stored. We need
/// the database namespace for the deployment as that information is only
/// stored in the primary database.
///
/// Any instance of this struct must originate in the database
pub struct Site {
    pub id: DeploymentId,
    /// The subgraph deployment
    pub deployment: DeploymentHash,
    /// The name of the database shard
    pub shard: Shard,
    /// The database namespace (schema) that holds the data for the deployment
    pub namespace: Namespace,
    /// The name of the network to which this deployment belongs
    pub network: String,
    /// Whether this is the site that should be used for queries. There's
    /// exactly one for each `deployment`, i.e., other entries for that
    /// deployment have `active = false`
    pub(crate) active: bool,
    /// Only the store and tests can create Sites
    _creation_disallowed: (),
}

/// A wrapper for a database connection that provides access to functionality
/// that works only on the primary database
pub struct Connection<'a>(MaybeOwned<'a, PooledConnection<ConnectionManager<PgConnection>>>);

impl<'a> Connection<'a> {
    pub fn new(
        conn: impl Into<MaybeOwned<'a, PooledConnection<ConnectionManager<PgConnection>>>>,
    ) -> Self {
        Self(conn.into())
    }

    pub(crate) fn transaction<T, E, F>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: From<diesel::result::Error>,
    {
        self.0.transaction(f)
    }

    /// Create a new subgraph with the given name. If one already exists, use
    /// the existing one. Return the `id` of the newly created or existing
    /// subgraph
    pub fn create_indexer(&self, name: &IndexerName) -> Result<String, StoreError> {
        use indexer as s;

        let conn = self.0.as_ref();
        let id = generate_entity_id();
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let inserted = insert_into(s::table)
            .values((
                s::id.eq(&id),
                s::name.eq(name.as_str()),
                // using BigDecimal::from(created_at) produced a scale error
                s::created_at.eq(sql(&format!("{}", created_at))),
                s::block_range.eq(UNVERSIONED_RANGE),
            ))
            .on_conflict(s::name)
            .do_nothing()
            .execute(conn)?;
        if inserted == 0 {
            let existing_id = s::table
                .filter(s::name.eq(name.as_str()))
                .select(s::id)
                .first::<String>(conn)?;
            Ok(existing_id)
        } else {
            Ok(id)
        }
    }

    pub fn find_site_by_ref(&self, id: DeploymentId) -> Result<Option<Site>, StoreError> {
        let schema = deployment_schemas::table
            .find(id)
            .first::<Schema>(self.0.as_ref())
            .optional()?;
        schema.map(|schema| schema.try_into()).transpose()
    }

    pub fn find_active_site(&self, indexer: &DeploymentHash) -> Result<Option<Site>, StoreError> {
        let schema = deployment_schemas::table
            .filter(deployment_schemas::indexer.eq(indexer.to_string()))
            .filter(deployment_schemas::active.eq(true))
            .first::<Schema>(self.0.as_ref())
            .optional()?;
        schema.map(|schema| schema.try_into()).transpose()
    }
}
