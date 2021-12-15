table! {
    indexer_deployments (id) {
        id -> Integer,
        hash -> Text,
        namespace -> Text,
        schema -> Text,
        failed -> Bool,
        health -> crate::deployment::IndexerHealthMapping,
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
