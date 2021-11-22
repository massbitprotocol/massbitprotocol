table! {
    indexers (v_id) {
        network -> Nullable<Varchar>,
        name -> Varchar,
        namespace -> Varchar,
        description -> Nullable<Varchar>,
        image_url -> Nullable<Varchar>,
        repo -> Nullable<Varchar>,
        manifest -> Varchar,
        mapping -> Varchar,
        graphql -> Varchar,
        status -> Nullable<Varchar>,
        address -> Nullable<Varchar>,
        start_block -> Int8,
        got_block -> Int8,
        version -> Nullable<Varchar>,
        hash -> Varchar,
        v_id -> Int8,
    }
}
