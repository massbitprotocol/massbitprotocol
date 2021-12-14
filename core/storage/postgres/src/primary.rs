//! Utilities for dealing with subgraph metadata that resides in the primary
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
use massbit_common::prelude::{anyhow::anyhow, lazy_static, serde_json};
use massbit_data::indexer::{DeploymentId as GraphDeploymentId, DeploymentLocator};
use massbit_data::store::entity::EntityChangeOperation;
// use graph::{
//     components::store::DeploymentLocator,
//     constraint_violation,
//     data::subgraph::status,
//     prelude::{
//         anyhow, bigdecimal::ToPrimitive, serde_json, DeploymentHash, EntityChange,
//         EntityChangeOperation, NodeId, StoreError, SubgraphName, SubgraphVersionSwitchingMode,
//     },
// };
// use graph::{data::subgraph::schema::generate_entity_id, prelude::StoreEvent};
use maybe_owned::MaybeOwned;
use std::{
    collections::HashMap,
    convert::TryFrom,
    convert::TryInto,
    fmt,
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    block_range::UNVERSIONED_RANGE,
    //detail::DeploymentDetail,
    //notification_listener::JsonNotification,
    indexer_store::{unused, Shard},
};

use massbit_data::constraint_violation;
use massbit_data::indexer::{DeploymentHash, NodeId};
use massbit_data::store::entity::EntityChange;
use massbit_data::store::{StoreError, StoreEvent};
#[cfg(debug_assertions)]
use std::sync::Mutex;
#[cfg(debug_assertions)]
lazy_static::lazy_static! {
    /// Tests set this to true so that `send_store_event` will store a copy
    /// of each event sent in `EVENT_TAP`
    pub static ref EVENT_TAP_ENABLED: Mutex<bool> = Mutex::new(false);
    pub static ref EVENT_TAP: Mutex<Vec<StoreEvent>> = Mutex::new(Vec::new());
}

table! {
    subgraphs.subgraph (vid) {
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
    subgraphs.subgraph_version (vid) {
        vid -> BigInt,
        id -> Text,
        subgraph -> Text,
        deployment -> Text,
        created_at -> Numeric,
        block_range -> Range<Integer>,
    }
}

table! {
    subgraphs.subgraph_deployment_assignment {
        id -> Integer,
        node_id -> Text,
    }
}

table! {
    active_copies(dst) {
        src -> Integer,
        dst -> Integer,
        queued_at -> Timestamptz,
        // Setting this column to a value signals to a running copy process
        // that a cancel has been requested. The copy process checks this
        // periodically and stops as soon as this is not null anymore
        cancelled_at -> Nullable<Timestamptz>,
    }
}

table! {
    public.ens_names(hash) {
        hash -> Varchar,
        name -> Varchar,
    }
}

/// We used to support different layout schemes. The old 'Split' scheme
/// which used JSONB layout has been removed, and we will only deal
/// with relational layout. Trying to do anything with a 'Split' subgraph
/// will result in an error.
#[derive(DbEnum, Debug, Clone, Copy)]
pub enum DeploymentSchemaVersion {
    Split,
    Relational,
}

table! {
    deployment_schemas(id) {
        id -> Integer,
        created_at -> Timestamptz,
        subgraph -> Text,
        name -> Text,
        shard -> Text,
        /// The subgraph layout scheme used for this subgraph
        version -> crate::primary::DeploymentSchemaVersionMapping,
        network -> Text,
        /// If there are multiple entries for the same IPFS hash (`subgraph`)
        /// only one of them will be active. That's the one we use for
        /// querying
        active -> Bool,
    }
}

table! {
    /// A table to track deployments that are no longer used. Once an unused
    /// deployment has been removed, the entry in this table is the only
    /// trace in the system that it ever existed
    unused_deployments(id) {
        // This is the same as what deployment_schemas.id was when the
        // deployment was still around
        id -> Integer,
        // The IPFS hash of the deployment
        deployment -> Text,
        // When we first detected that the deployment was unused
        unused_at -> Timestamptz,
        // When we actually deleted the deployment
        removed_at -> Nullable<Timestamptz>,
        // When the deployment was created
        created_at -> Timestamptz,
        /// Data that we get from the primary
        subgraphs -> Nullable<Array<Text>>,
        namespace -> Text,
        shard -> Text,

        /// Data we fill in from the deployment's shard
        entity_count -> Integer,
        latest_ethereum_block_hash -> Nullable<Binary>,
        latest_ethereum_block_number -> Nullable<Integer>,
        failed -> Bool,
        synced -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(
    subgraph,
    subgraph_version,
    subgraph_deployment_assignment,
    deployment_schemas,
    unused_deployments,
    active_copies,
);

/// Information about the database schema that stores the entities for a
/// subgraph.
#[derive(Clone, Queryable, QueryableByName, Debug)]
#[table_name = "deployment_schemas"]
struct Schema {
    id: DeploymentId,
    pub created_at: PgTimestamp,
    pub subgraph: String,
    pub name: String,
    pub shard: String,
    /// The version currently in use. Always `Relational`, attempts to load
    /// schemas from the database with `Split` produce an error
    version: DeploymentSchemaVersion,
    pub network: String,
    pub(crate) active: bool,
}

#[derive(Clone, Queryable, QueryableByName, Debug)]
#[table_name = "unused_deployments"]
pub struct UnusedDeployment {
    pub id: DeploymentId,
    pub deployment: String,
    pub unused_at: PgTimestamp,
    pub removed_at: Option<PgTimestamp>,
    pub created_at: PgTimestamp,
    pub subgraphs: Option<Vec<String>>,
    pub namespace: String,
    pub shard: String,

    /// Data we fill in from the deployment's shard
    pub entity_count: i32,
    pub latest_ethereum_block_hash: Option<Vec<u8>>,
    pub latest_ethereum_block_number: Option<i32>,
    pub failed: bool,
    pub synced: bool,
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
        self.0.fmt(f)
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
        self.0.fmt(f)
    }
}

impl From<DeploymentId> for GraphDeploymentId {
    fn from(id: DeploymentId) -> Self {
        GraphDeploymentId::new(id.0)
    }
}

impl From<GraphDeploymentId> for DeploymentId {
    fn from(id: GraphDeploymentId) -> Self {
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

impl Site {
    pub fn new(hash: DeploymentHash, shard: Shard, schema: String) -> Self {
        Self {
            id: DeploymentId(0),
            deployment: hash,
            shard,
            namespace: Namespace::new(schema).unwrap(),
            network: "mainnet".to_string(),
            active: true,
            _creation_disallowed: (),
        }
    }
}
impl TryFrom<Schema> for Site {
    type Error = StoreError;

    fn try_from(schema: Schema) -> Result<Self, Self::Error> {
        if matches!(schema.version, DeploymentSchemaVersion::Split) {
            return Err(constraint_violation!(
                "the subgraph {} uses JSONB layout which is not supported any longer",
                schema.subgraph.as_str()
            ));
        }
        let deployment = DeploymentHash::new(&schema.subgraph)
            .map_err(|s| constraint_violation!("Invalid deployment id {}", s))?;
        let namespace = Namespace::new(schema.name.clone()).map_err(|nsp| {
            constraint_violation!(
                "Invalid schema name {} for deployment {}",
                nsp,
                &schema.subgraph
            )
        })?;
        let shard = Shard::new(schema.shard)?;
        Ok(Self {
            id: schema.id,
            deployment,
            namespace,
            shard,
            network: schema.network,
            active: schema.active,
            _creation_disallowed: (),
        })
    }
}

impl From<&Site> for DeploymentLocator {
    fn from(site: &Site) -> Self {
        DeploymentLocator::new(site.id.into(), site.deployment.clone())
    }
}

/// This is only used for tests to allow them to create a `Site` that does
/// not originate in the database
#[cfg(debug_assertions)]
pub fn make_dummy_site(deployment: DeploymentHash, namespace: Namespace, network: String) -> Site {
    use crate::PRIMARY_SHARD;

    Site {
        id: DeploymentId(-7),
        deployment,
        shard: PRIMARY_SHARD.clone(),
        namespace,
        network,
        active: true,
        _creation_disallowed: (),
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
    // pub fn find_active_site(&self, hash: &DeploymentHash) -> Result<Option<Site>, StoreError> {
    //     let schema = deployment_schemas::table
    //         .filter(deployment_schemas::subgraph.eq(subgraph.to_string()))
    //         .filter(deployment_schemas::active.eq(true))
    //         .first::<Schema>(self.0.as_ref())
    //         .optional()?;
    //     schema.map(|schema| schema.try_into()).transpose()
    // }
}
