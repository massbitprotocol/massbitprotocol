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

use indexer_orm::models::{DeploymentId, DeploymentSchema, Namespace};
use indexer_orm::schema::indexer_deployment_schemas;
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

// allow_tables_to_appear_in_same_query!(
//     subgraph,
//     subgraph_version,
//     subgraph_deployment_assignment,
//     deployment_schemas,
//     unused_deployments,
//     active_copies,
// );

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
            id: DeploymentId::new(0),
            deployment: hash,
            shard,
            namespace: Namespace::new(schema).unwrap(),
            network: "mainnet".to_string(),
            active: true,
            _creation_disallowed: (),
        }
    }
}
impl TryFrom<DeploymentSchema> for Site {
    type Error = StoreError;

    fn try_from(schema: DeploymentSchema) -> Result<Self, Self::Error> {
        let deployment = DeploymentHash::new(&schema.indexer_hash)
            .map_err(|s| constraint_violation!("Invalid deployment id {}", s))?;
        let namespace = Namespace::new(schema.schema_name.clone()).map_err(|nsp| {
            constraint_violation!(
                "Invalid schema name {} for deployment {}",
                nsp,
                &schema.indexer_hash
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
        id: DeploymentId::new(-7),
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
    pub fn find_active_site(&self, hash: &DeploymentHash) -> Result<Option<Site>, StoreError> {
        let schema = indexer_deployment_schemas::table
            .filter(indexer_deployment_schemas::indexer_hash.eq(hash.to_string()))
            .filter(indexer_deployment_schemas::active.eq(true))
            .first::<DeploymentSchema>(self.0.as_ref())
            .optional()?;
        schema.map(|schema| schema.try_into()).transpose()
    }
}
