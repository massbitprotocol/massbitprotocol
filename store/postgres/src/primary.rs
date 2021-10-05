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
use massbit::components::store::{DeploymentId as IndexerDeploymentId, DeploymentLocator};
use massbit::constraint_violation;
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
use massbit::data::indexer::schema::generate_entity_id;

table! {
    indexer (vid) {
        vid -> BigInt,
        id -> Text,
        name -> Text,
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
    }
}

allow_tables_to_appear_in_same_query!(indexer, deployment_schemas);

/// Information about the database schema that stores the entities for a
/// indexer.
#[derive(Clone, Queryable, QueryableByName, Debug)]
#[table_name = "deployment_schemas"]
struct Schema {
    pub id: DeploymentId,
    pub created_at: PgTimestamp,
    pub indexer: String,
    pub name: String,
    pub shard: String,
    pub network: String,
}

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
    /// The indexer deployment
    pub deployment: DeploymentHash,
    /// The name of the database shard
    pub shard: Shard,
    /// The database namespace (schema) that holds the data for the deployment
    pub namespace: Namespace,
    /// The name of the network to which this deployment belongs
    pub network: String,
    /// Only the store and tests can create Sites
    _creation_disallowed: (),
}

impl TryFrom<Schema> for Site {
    type Error = StoreError;

    fn try_from(schema: Schema) -> Result<Self, Self::Error> {
        let deployment = DeploymentHash::new(&schema.indexer)
            .map_err(|s| constraint_violation!("Invalid deployment id {}", s))?;
        let namespace = Namespace::new(schema.name.clone()).map_err(|nsp| {
            constraint_violation!(
                "Invalid schema name {} for deployment {}",
                nsp,
                &schema.indexer
            )
        })?;
        let shard = Shard::new(schema.shard)?;
        Ok(Self {
            id: schema.id,
            deployment,
            namespace,
            shard,
            network: schema.network,
            _creation_disallowed: (),
        })
    }
}

impl From<&Site> for DeploymentLocator {
    fn from(site: &Site) -> Self {
        DeploymentLocator::new(site.id.into(), site.deployment.clone())
    }
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

    /// Create a new indexer with the given name. If one already exists, use
    /// the existing one. Return the `id` of the newly created or existing
    /// indexer
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

    pub fn indexer_exists(&self, name: &IndexerName) -> Result<bool, StoreError> {
        use indexer as s;

        Ok(
            diesel::select(exists(s::table.filter(s::name.eq(name.as_str()))))
                .get_result::<bool>(self.0.as_ref())?,
        )
    }

    pub fn find_site_by_ref(&self, id: DeploymentId) -> Result<Option<Site>, StoreError> {
        let schema = deployment_schemas::table
            .find(id)
            .first::<Schema>(self.0.as_ref())
            .optional()?;
        schema.map(|schema| schema.try_into()).transpose()
    }

    /// Create a new site and possibly set it to the active site. This
    /// function only performs the basic operations for creation, and the
    /// caller must check that other conditions (like whether there already
    /// is an active site for the deployment) are met
    fn create_site(
        &self,
        shard: Shard,
        deployment: DeploymentHash,
        network: String,
    ) -> Result<Site, StoreError> {
        use deployment_schemas as ds;

        let conn = self.0.as_ref();

        let schemas: Vec<(DeploymentId, String)> = diesel::insert_into(ds::table)
            .values((
                ds::indexer.eq(deployment.as_str()),
                ds::shard.eq(shard.as_str()),
                ds::network.eq(network.as_str()),
            ))
            .returning((ds::id, ds::name))
            .get_results(conn)?;
        let (id, namespace) = schemas
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("failed to read schema name for {} back", deployment))?;
        let namespace = Namespace::new(namespace).map_err(|name| {
            constraint_violation!("Generated database schema name {} is invalid", name)
        })?;

        Ok(Site {
            id,
            deployment,
            shard,
            namespace,
            network,
            _creation_disallowed: (),
        })
    }

    pub fn allocate_site(
        &self,
        shard: Shard,
        indexer: &DeploymentHash,
        network: String,
    ) -> Result<Site, StoreError> {
        if let Some(site) = self.find_site(indexer)? {
            return Ok(site);
        }

        self.create_site(shard, indexer.clone(), network)
    }

    /// Find sites by their indexer deployment hashes. If `ids` is empty,
    /// return all sites
    pub fn find_sites(&self, ids: Vec<String>) -> Result<Vec<Site>, StoreError> {
        use deployment_schemas as ds;

        let schemas = if ids.is_empty() {
            ds::table.load::<Schema>(self.0.as_ref())?
        } else {
            ds::table
                .filter(ds::indexer.eq_any(ids))
                .load::<Schema>(self.0.as_ref())?
        };
        schemas
            .into_iter()
            .map(|schema| schema.try_into())
            .collect()
    }

    pub fn find_site(&self, indexer: &DeploymentHash) -> Result<Option<Site>, StoreError> {
        let schema = deployment_schemas::table
            .filter(deployment_schemas::indexer.eq(indexer.to_string()))
            .first::<Schema>(self.0.as_ref())
            .optional()?;
        schema.map(|schema| schema.try_into()).transpose()
    }
}
