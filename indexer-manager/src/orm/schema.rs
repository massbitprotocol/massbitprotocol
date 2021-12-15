table! {
    use diesel::sql_types::{Nullable,Varchar,Bool,Int8};
    use crate::orm::IndexerStatusMapping;
    indexers (v_id) {
        network -> Nullable<Varchar>,
        name -> Varchar,
        namespace -> Varchar,
        description -> Nullable<Varchar>,
        image_url -> Nullable<Varchar>,
        repository -> Nullable<Varchar>,
        manifest -> Varchar,
        mapping -> Varchar,
        graphql -> Varchar,
        status -> IndexerStatusMapping,
        deleted -> Bool,
        address -> Nullable<Varchar>,
        start_block -> Int8,
        got_block -> Int8,
        version -> Nullable<Varchar>,
        hash -> Varchar,
        v_id -> Int8,
    }
}
