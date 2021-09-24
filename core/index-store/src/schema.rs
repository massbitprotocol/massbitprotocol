table! {
    indexers (v_id) {
        id -> Text,
        network -> Text,
        name -> Text,
        namespace -> Text,
        description -> Text,
        repo -> Text,
        manifest -> Text,
        index_status -> Text,
        got_block -> Int8,
        hash -> Text,
        v_id -> Int4,
    }
}