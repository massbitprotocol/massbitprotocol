table! {
    use diesel::sql_types::{Nullable,Varchar,Bool,Int8};
    use crate::models::IndexerStatusMapping;
    indexers (v_id) {
        network -> Nullable<Varchar>,
        name -> Varchar,
        namespace -> Varchar,
        description -> Nullable<Varchar>,
        image_url -> Nullable<Varchar>,
        repository -> Nullable<Varchar>,
        manifest -> Varchar,
        mapping -> Varchar,
        unpack_instruction -> Varchar,
        graphql -> Varchar,
        status -> IndexerStatusMapping,
        deleted -> Bool,
        address -> Nullable<Varchar>,
        start_block -> Int8,
        got_block -> Int8,
        version -> Nullable<Varchar>,
        hash -> Varchar,
        owner_id -> Varchar,
        v_id -> Int8,
    }
}

table! {
    indexer_deployments (id) {
        id -> Integer,
        hash -> Text,
        namespace -> Text,
        schema -> Text,
        failed -> Bool,
        health -> crate::models::IndexerHealthMapping,
        synced -> Bool,
        fatal_error -> Nullable<Text>,
        non_fatal_errors -> Array<Text>,
        earliest_block_hash -> Nullable<Binary>,
        earliest_block_number -> Nullable<Numeric>,
        latest_block_hash -> Nullable<Binary>,
        latest_block_number -> Nullable<Numeric>,
        last_healthy_block_hash -> Nullable<Binary>,
        last_healthy_block_number -> Nullable<Numeric>,
        entity_count -> Numeric,
        graft_base -> Nullable<Text>,
        graft_block_hash -> Nullable<Binary>,
        graft_block_number -> Nullable<Numeric>,
        reorg_count -> Integer,
        current_reorg_depth -> Integer,
        max_reorg_depth -> Integer,
    }
}

table! {
    indexer_deployment_schemas(id) {
        id -> Integer,
        created_at -> Timestamptz,
        indexer_hash -> Text,
        schema_name -> Text,
        shard -> Text,
        network -> Text,
        /// If there are multiple entries for the same IPFS hash (`subgraph`)
        /// only one of them will be active. That's the one we use for
        /// querying
        active -> Bool,
    }
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
table! {
    public.ens_names(hash) {
        hash -> Varchar,
        name -> Varchar,
    }
}

allow_tables_to_appear_in_same_query!(
    subgraph,
    subgraph_version,
    subgraph_deployment_assignment,
    indexer_deployment_schemas,
    unused_deployments,
    active_copies,
);
