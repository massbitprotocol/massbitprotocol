table! {
    daily_matic_address_transaction (id) {
        id -> Int4,
        address -> Nullable<Text>,
        transaction_date -> Date,
        transaction_count -> Numeric,
        transaction_volume -> Numeric,
        gas -> Numeric,
    }
}

table! {
    daily_transaction (id) {
        id -> Int4,
        network -> Varchar,
        transaction_date -> Date,
        transaction_count -> Numeric,
        transaction_volume -> Numeric,
        gas -> Numeric,
        average_gas_price -> Numeric,
    }
}

table! {
    matic_block (block_hash) {
        block_hash -> Text,
        block_number -> Nullable<Int8>,
        transaction_number -> Nullable<Int8>,
        timestamp -> Int8,
        validated_by -> Nullable<Text>,
        reward -> Nullable<Numeric>,
        difficulty -> Nullable<Numeric>,
        total_difficulty -> Nullable<Numeric>,
        size -> Nullable<Int8>,
        gas_used -> Nullable<Numeric>,
        gas_limit -> Nullable<Numeric>,
        extra_data -> Nullable<Bytea>,
    }
}

table! {
    matic_transaction (transaction_hash) {
        transaction_hash -> Text,
        block_hash -> Nullable<Text>,
        block_number -> Nullable<Int8>,
        nonce -> Nullable<Numeric>,
        sender -> Text,
        receiver -> Nullable<Text>,
        value -> Numeric,
        gas -> Numeric,
        gas_price -> Numeric,
        timestamp -> Int8,
    }
}

allow_tables_to_appear_in_same_query!(
    daily_matic_address_transaction,
    daily_transaction,
    matic_block,
    matic_transaction,
);
