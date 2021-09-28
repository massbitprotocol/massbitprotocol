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

use massbit::prelude::{Schema, StoreError};

use crate::primary::Site;

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
    use indexer_manifest as im;
    let s: String = im::table
        .select(im::schema)
        .filter(im::id.eq(site.id))
        .first(conn)?;
    Schema::parse(s.as_str(), site.deployment.clone()).map_err(|e| StoreError::Unknown(e))
}
