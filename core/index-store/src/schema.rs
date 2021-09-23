table! {
    indexer_state (id) {
        id -> Int4,
        indexer_hash -> Text,
        schema_name -> Text,
        got_block -> Int8,
    }
}
