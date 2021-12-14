use crate::orm::schema::indexer_deployments;
use crate::primary::Site;
use diesel::dsl::sql;
use diesel::pg::PgConnection;
use diesel::query_dsl::methods::{FilterDsl, SelectDsl};
use diesel::{ExpressionMethods, OptionalExtension, RunQueryDsl};
use massbit_common::prelude::anyhow::anyhow;
use massbit_common::prelude::bigdecimal::ToPrimitive;
use massbit_data::constraint_violation;
use massbit_data::indexer::{DeploymentHash, DeploymentState, IndexerFeature};
use massbit_data::prelude::StoreError;
use massbit_data::schema::Schema;
use massbit_data::store::chain::BlockHash;
use massbit_data::store::scalar::BigDecimal;
use massbit_data::store::{BlockNumber, BlockPtr};
use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::io::Bytes;
use std::ops::Deref;

#[derive(DbEnum, Debug, Clone, Copy)]
pub enum IndexerHealth {
    Failed,
    Healthy,
    Unhealthy,
}

/// Look up the graft point for the given subgraph in the database and
/// return it. If `pending_only` is `true`, only return `Some(_)` if the
/// deployment has not progressed past the graft point, i.e., data has not
/// been copied for the graft
fn graft(
    conn: &PgConnection,
    hash: &DeploymentHash,
    pending_only: bool,
) -> Result<Option<(DeploymentHash, BlockPtr)>, StoreError> {
    use indexer_deployments as d;

    let graft_query = d::table
        .select((d::graft_base, d::graft_block_hash, d::graft_block_number))
        .filter(d::hash.eq(hash.as_str()));
    // The name of the base subgraph, the hash, and block number
    let graft: (Option<String>, Option<Vec<u8>>, Option<BigDecimal>) = if pending_only {
        graft_query
            .filter(d::graft_block_number.ge(sql("coalesce(latest_block_number, 0)")))
            .first(conn)
            .optional()?
            .unwrap_or((None, None, None))
    } else {
        graft_query
            .first(conn)
            .optional()?
            .unwrap_or((None, None, None))
    };
    match graft {
        (None, None, None) => Ok(None),
        (Some(indexer), Some(hash), Some(block)) => {
            let block = block.to_i64().expect("block numbers fit into a i64");
            let indexer = DeploymentHash::new(indexer.clone()).map_err(|_| {
                StoreError::Unknown(anyhow!(
                    "the base subgraph for a graft must be a valid subgraph id but is `{}`",
                    indexer
                ))
            })?;
            Ok(Some((indexer, BlockPtr::from((hash, block)))))
        }
        _ => unreachable!(
            "graftBlockHash and graftBlockNumber are either both set or neither is set"
        ),
    }
}

/// Look up the graft point for the given subgraph in the database and
/// return it. Returns `None` if the deployment does not have
/// a graft or if the subgraph has already progress past the graft point,
/// indicating that the data copying for grafting has been performed
pub fn graft_pending(
    conn: &PgConnection,
    hash: &DeploymentHash,
) -> Result<Option<(DeploymentHash, BlockPtr)>, StoreError> {
    graft(conn, hash, true)
}

/// Look up the graft point for the given subgraph in the database and
/// return it. Returns `None` if the deployment does not have
/// a graft
pub fn graft_point(
    conn: &PgConnection,
    hash: &DeploymentHash,
) -> Result<Option<(DeploymentHash, BlockPtr)>, StoreError> {
    graft(conn, hash, false)
}

pub fn schema(conn: &PgConnection, site: &Site) -> Result<Schema, StoreError> {
    use indexer_deployments as d;
    let s: String = d::table
        .select(d::schema)
        .filter(d::hash.eq(site.deployment.deref()))
        .first(conn)?;
    Schema::parse(s.as_str(), site.deployment.clone()).map_err(|e| StoreError::Unknown(e))
}

/// Returns `true` if the deployment `indexer_hash` exists and is synced
pub fn exists_and_synced(conn: &PgConnection, hash: &str) -> Result<bool, StoreError> {
    use indexer_deployments as d;

    let synced = d::table
        .filter(d::hash.eq(hash))
        .select(d::synced)
        .first(conn)
        .optional()?
        .unwrap_or(false);
    Ok(synced)
}

pub fn block_ptr(
    conn: &PgConnection,
    hash: &DeploymentHash,
) -> Result<Option<BlockPtr>, StoreError> {
    use indexer_deployments as d;

    let ptr = match d::table
        .filter(d::hash.eq(hash.as_str()))
        .select((d::latest_block_number, d::latest_block_hash))
        .first::<(Option<BigDecimal>, Option<Vec<u8>>)>(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => StoreError::DeploymentNotFound(hash.to_string()),
            e => e.into(),
        })? {
        (Some(number), Some(hash)) => Some(BlockPtr {
            hash: BlockHash::from(hash),
            number: number.to_i64().unwrap_or_default(),
        }),
        (None, None) => None,
        _ => None,
    };
    Ok(ptr)
}

pub fn state(conn: &PgConnection, hash: DeploymentHash) -> Result<DeploymentState, StoreError> {
    use indexer_deployments as d;
    match d::table
        .filter(d::hash.eq(hash.as_str()))
        .select((
            d::hash,
            d::reorg_count,
            d::max_reorg_depth,
            d::latest_block_number,
        ))
        .first::<(String, i32, i32, Option<BigDecimal>)>(conn)
        .optional()?
    {
        None => Err(StoreError::QueryExecutionError(format!(
            "No data found for indexer {}",
            hash
        ))),
        Some((_, reorg_count, max_reorg_depth, latest_block_number)) => {
            let reorg_count = convert_to_u32(Some(reorg_count), "reorg_count", hash.as_str())?;
            let max_reorg_depth =
                convert_to_u32(Some(max_reorg_depth), "max_reorg_depth", hash.as_str())?;
            let latest_block_number = latest_as_block_number(latest_block_number, hash.as_str())?;

            Ok(DeploymentState {
                hash,
                reorg_count,
                max_reorg_depth,
                latest_block_number,
            })
        }
    }
}

fn convert_to_u32(number: Option<i32>, field: &str, indexer_hash: &str) -> Result<u32, StoreError> {
    number
        .ok_or_else(|| constraint_violation!("missing {} for indexer `{}`", field, indexer_hash))
        .and_then(|number| {
            u32::try_from(number).map_err(|_| {
                constraint_violation!(
                    "invalid value {:?} for {} in subgraph {}",
                    number,
                    field,
                    indexer_hash
                )
            })
        })
}

/// Translate `latest` into a `BlockNumber`. If `latest` is `None` or does
/// not represent an `i32`, return an error
fn latest_as_block_number(
    latest: Option<BigDecimal>,
    indexer_hash: &str,
) -> Result<BlockNumber, StoreError> {
    match latest {
        None => Err(StoreError::QueryExecutionError(format!(
            "Indexer `{}` has not started syncing yet. Wait for it to ingest \
             a few blocks before querying it",
            indexer_hash
        ))),
        Some(latest) => latest.to_i64().ok_or_else(|| {
            constraint_violation!(
                "Indexer `{}` has an \
                 invalid latest_block_number `{:?}` that can not be \
                 represented as an i32",
                indexer_hash,
                latest
            )
        }),
    }
}

pub fn manifest_info(conn: &PgConnection, site: &Site) -> Result<Schema, StoreError> {
    use indexer_deployments as d;
    let s: String = d::table
        .select(d::schema)
        .filter(d::hash.eq(site.deployment.as_str()))
        .first(conn)?;
    Schema::parse(s.as_str(), site.deployment.clone())
        .map_err(|e| StoreError::Unknown(e))
        .map(|schema| schema)
}

// pub fn features(conn: &PgConnection, site: &Site) -> Result<BTreeSet<IndexerFeature>, StoreError> {
//     use indexer_deployments as d;
//
//     let features: Vec<String> = d::table
//         .select(d::features)
//         .filter(d::id.eq(site.id))
//         .first(conn)
//         .unwrap();
//     features
//         .iter()
//         .map(|f| IndexerFeature::from_str(f).map_err(StoreError::from))
//         .collect()
// }

/// If `block` is `None`, assumes the latest block.
pub fn has_non_fatal_errors(
    conn: &PgConnection,
    hash: &DeploymentHash,
    block: Option<BlockNumber>,
) -> Result<bool, StoreError> {
    use indexer_deployments as d;
    // match block {
    //     Some(block) => select(diesel::dsl::exists(
    //         e::table
    //             .filter(e::subgraph_id.eq(id.as_str()))
    //             .filter(e::deterministic)
    //             .filter(sql("block_range @> ").bind::<Integer, _>(block)),
    //     ))
    //         .get_result(conn),
    //     None => select(diesel::dsl::exists(
    //         e::table
    //             .filter(e::subgraph_id.eq(id.as_str()))
    //             .filter(e::deterministic)
    //             .filter(
    //                 sql("block_range @> ")
    //                     .bind(
    //                         d::table
    //                             .filter(d::deployment.eq(id.as_str()))
    //                             .select(d::latest_ethereum_block_number)
    //                             .single_value(),
    //                     )
    //                     .sql("::int"),
    //             ),
    //     ))
    //         .get_result(conn),
    // }
    //     .map_err(|e| e.into())
    //Todo: handle non fatal errors
    Ok(false)
}
