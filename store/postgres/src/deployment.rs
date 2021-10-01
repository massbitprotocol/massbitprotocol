use diesel::{
    connection::SimpleConnection,
    dsl::{count, delete, insert_into, select, sql, update},
    sql_types::Integer,
};
use diesel::{expression::SqlLiteral, pg::PgConnection, sql_types::Numeric};
use diesel::{
    prelude::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl},
    sql_query,
    sql_types::{Nullable, Text},
};

use massbit::constraint_violation;
use massbit::data::indexer::schema::{IndexerDeploymentEntity, IndexerManifestEntity};
use massbit::prelude::*;
use massbit::prelude::{BlockPtr, DeploymentHash, Schema, StoreError};

use crate::primary::Site;

table! {
    indexer_deployment (id) {
        id -> Integer,
        deployment -> Text,
        failed -> Bool,
        synced -> Bool,
        fatal_error -> Nullable<Text>,
        non_fatal_errors -> Array<Text>,
        earliest_ethereum_block_hash -> Nullable<Binary>,
        earliest_ethereum_block_number -> Nullable<Numeric>,
        latest_ethereum_block_hash -> Nullable<Binary>,
        latest_ethereum_block_number -> Nullable<Numeric>,
        last_healthy_ethereum_block_hash -> Nullable<Binary>,
        last_healthy_ethereum_block_number -> Nullable<Numeric>,
        entity_count -> Numeric,
        reorg_count -> Integer,
        current_reorg_depth -> Integer,
        max_reorg_depth -> Integer,
        firehose_cursor -> Nullable<Text>,
    }
}

table! {
    indexer_manifest {
        id -> Integer,
        spec_version -> Text,
        description -> Nullable<Text>,
        repository -> Nullable<Text>,
        features -> Array<Text>,
        schema -> Text,
    }
}

pub fn schema(conn: &PgConnection, site: &Site) -> Result<Schema, StoreError> {
    use indexer_manifest as sm;
    let s: String = sm::table
        .select(sm::schema)
        .filter(sm::id.eq(site.id))
        .first(conn)?;
    Schema::parse(s.as_str(), site.deployment.clone()).map_err(|e| StoreError::Unknown(e))
}

pub fn create_deployment(
    conn: &PgConnection,
    site: &Site,
    deployment: IndexerDeploymentEntity,
    exists: bool,
    replace: bool,
) -> Result<(), StoreError> {
    use indexer_deployment as d;
    use indexer_manifest as m;

    fn b(ptr: &Option<BlockPtr>) -> Option<&[u8]> {
        ptr.as_ref().map(|ptr| ptr.hash_slice())
    }

    fn n(ptr: &Option<BlockPtr>) -> SqlLiteral<Nullable<Numeric>> {
        match ptr {
            None => sql("null"),
            Some(ptr) => sql(&format!("{}::numeric", ptr.number)),
        }
    }

    let IndexerDeploymentEntity {
        manifest:
            IndexerManifestEntity {
                spec_version,
                description,
                repository,
                features,
                schema,
            },
        failed,
        synced,
        fatal_error: _,
        non_fatal_errors: _,
        earliest_block,
        latest_block,
        reorg_count: _,
        current_reorg_depth: _,
        max_reorg_depth: _,
    } = deployment;

    let deployment_values = (
        d::id.eq(site.id),
        d::deployment.eq(site.deployment.as_str()),
        d::failed.eq(failed),
        d::synced.eq(synced),
        d::fatal_error.eq::<Option<String>>(None),
        d::non_fatal_errors.eq::<Vec<String>>(vec![]),
        d::earliest_ethereum_block_hash.eq(b(&earliest_block)),
        d::earliest_ethereum_block_number.eq(n(&earliest_block)),
        d::latest_ethereum_block_hash.eq(b(&latest_block)),
        d::latest_ethereum_block_number.eq(n(&latest_block)),
        d::entity_count.eq(sql("0")),
    );

    let manifest_values = (
        m::id.eq(site.id),
        m::spec_version.eq(spec_version),
        m::description.eq(description),
        m::repository.eq(repository),
        m::features.eq(features),
        m::schema.eq(schema),
    );

    if exists && replace {
        update(d::table.filter(d::deployment.eq(site.deployment.as_str())))
            .set(deployment_values)
            .execute(conn)?;

        update(m::table.filter(m::id.eq(site.id)))
            .set(manifest_values)
            .execute(conn)?;
    } else {
        insert_into(d::table)
            .values(deployment_values)
            .execute(conn)?;

        insert_into(m::table)
            .values(manifest_values)
            .execute(conn)?;
    }
    Ok(())
}

/// Returns `true` if the deployment (as identified by `site.id`)
pub fn exists(conn: &PgConnection, site: &Site) -> Result<bool, StoreError> {
    use indexer_deployment as d;

    let exists = d::table
        .filter(d::id.eq(site.id))
        .count()
        .get_result::<i64>(conn)?
        > 0;
    Ok(exists)
}

pub fn forward_block_ptr(
    conn: &PgConnection,
    id: &DeploymentHash,
    ptr: BlockPtr,
) -> Result<(), StoreError> {
    use crate::diesel::BoolExpressionMethods;
    use indexer_deployment as d;

    // Work around a Diesel issue with serializing BigDecimals to numeric
    let number = format!("{}::numeric", ptr.number);

    let row_count = update(
        d::table.filter(d::deployment.eq(id.as_str())).filter(
            // Asserts that the processing direction is forward.
            d::latest_ethereum_block_number
                .lt(sql(&number))
                .or(d::latest_ethereum_block_number.is_null()),
        ),
    )
    .set((
        d::latest_ethereum_block_number.eq(sql(&number)),
        d::latest_ethereum_block_hash.eq(ptr.hash_slice()),
        d::current_reorg_depth.eq(0),
    ))
    .execute(conn)
    .map_err(StoreError::from)?;

    match row_count {
        // Common case: A single row was updated.
        1 => Ok(()),

        // No matching rows were found. This is an error. By the filter conditions, this can only be
        // due to a missing deployment (which `block_ptr` catches) or duplicate block processing.
        0 => match block_ptr(&conn, id)? {
            Some(block_ptr_from) if block_ptr_from.number >= ptr.number => {
                Err(StoreError::DuplicateBlockProcessing(id.clone(), ptr.number))
            }
            None | Some(_) => Err(StoreError::Unknown(anyhow!(
                "unknown error forwarding block ptr"
            ))),
        },

        // More than one matching row was found.
        _ => Err(StoreError::ConstraintViolation(
            "duplicate deployments in shard".to_owned(),
        )),
    }
}

pub fn block_ptr(conn: &PgConnection, id: &DeploymentHash) -> Result<Option<BlockPtr>, StoreError> {
    use indexer_deployment as d;

    let (number, hash) = d::table
        .filter(d::deployment.eq(id.as_str()))
        .select((
            d::latest_ethereum_block_number,
            d::latest_ethereum_block_hash,
        ))
        .first::<(Option<BigDecimal>, Option<Vec<u8>>)>(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => StoreError::DeploymentNotFound(id.to_string()),
            e => e.into(),
        })?;

    let ptr = crate::detail::block(id.as_str(), "latest_ethereum_block", hash, number)?
        .map(|block| block.to_ptr());
    Ok(ptr)
}
