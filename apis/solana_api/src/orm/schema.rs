table! {
    ethereum_blocks (block_hash) {
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
        parent_hash -> Nullable<Varchar>,
    }
}

table! {
    ethereum_daily_address_transactions (id) {
        id -> Int4,
        address -> Nullable<Text>,
        transaction_date -> Varchar,
        transaction_count -> Numeric,
        transaction_volume -> Numeric,
        gas -> Numeric,
        timestamp -> Nullable<Int8>,
    }
}

table! {
    ethereum_daily_transactions (id) {
        id -> Int4,
        network -> Varchar,
        transaction_date -> Varchar,
        transaction_count -> Numeric,
        transaction_volume -> Numeric,
        gas -> Numeric,
        average_gas_price -> Numeric,
        timestamp -> Nullable<Int8>,
    }
}

table! {
    ethereum_transactions (transaction_hash) {
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

table! {
    network_states (id) {
        id -> Int8,
        chain -> Text,
        network -> Text,
        got_block -> Int8,
    }
}

table! {
    solana_account_transactions (id) {
        block_slot -> Nullable<Int8>,
        tx_index -> Nullable<Int2>,
        account -> Nullable<Varchar>,
        pre_balance -> Nullable<Int8>,
        post_balance -> Nullable<Int8>,
        id -> Int8,
    }
}

table! {
    solana_blocks (block_hash) {
        block_slot -> Nullable<Int8>,
        block_hash -> Varchar,
        previous_block_hash -> Nullable<Varchar>,
        parent_slot -> Nullable<Int8>,
        transaction_number -> Nullable<Int8>,
        timestamp -> Nullable<Int8>,
        leader -> Nullable<Varchar>,
        reward -> Nullable<Int8>,
    }
}

table! {
    solana_daily_stat_blocks (id) {
        id -> Int8,
        network -> Nullable<Varchar>,
        date -> Nullable<Int8>,
        min_block_slot -> Nullable<Int8>,
        max_block_slot -> Nullable<Int8>,
        block_counter -> Nullable<Int8>,
        total_tx -> Nullable<Int8>,
        success_tx -> Nullable<Int8>,
        total_reward -> Nullable<Int8>,
        total_fee -> Nullable<Int8>,
        average_block_time -> Nullable<Int8>,
        fist_block_time -> Nullable<Int8>,
        last_block_time -> Nullable<Int8>,
    }
}

table! {
    solana_inner_instructions (id) {
        id -> Int8,
    }
}

table! {
    solana_inst_advance_nonces (id) {
        id -> Int8,
        tx_hash -> Nullable<Varchar>,
        block_time -> Nullable<Int8>,
        inst_order -> Nullable<Int4>,
        nonce_account -> Nullable<Varchar>,
        recent_block_hashes_sysvar -> Nullable<Varchar>,
        nonce_authority -> Nullable<Varchar>,
    }
}

table! {
    solana_inst_allocates (id) {
        id -> Int8,
        tx_hash -> Nullable<Varchar>,
        block_time -> Nullable<Int8>,
        inst_order -> Nullable<Int4>,
        account -> Nullable<Varchar>,
        space -> Nullable<Int8>,
        base -> Nullable<Varchar>,
        seed -> Nullable<Text>,
        owner -> Nullable<Varchar>,
    }
}

table! {
    solana_inst_assigns (id) {
        id -> Int8,
        tx_hash -> Nullable<Varchar>,
        block_time -> Nullable<Int8>,
        inst_order -> Nullable<Int4>,
        account -> Nullable<Varchar>,
        base -> Nullable<Varchar>,
        seed -> Nullable<Text>,
        owner -> Nullable<Varchar>,
    }
}

table! {
    solana_inst_authorize_nonces (id) {
        id -> Int8,
        tx_hash -> Nullable<Varchar>,
        block_time -> Nullable<Int8>,
        inst_order -> Nullable<Int4>,
        nonce_account -> Nullable<Varchar>,
        nonce_authority -> Nullable<Varchar>,
        new_authorized -> Nullable<Varchar>,
    }
}

table! {
    solana_inst_create_accounts (id) {
        id -> Int8,
        tx_hash -> Nullable<Varchar>,
        block_time -> Nullable<Int8>,
        inst_order -> Nullable<Int4>,
        source -> Nullable<Varchar>,
        new_account -> Nullable<Varchar>,
        base -> Nullable<Varchar>,
        seed -> Nullable<Text>,
        lamports -> Nullable<Int8>,
        space -> Nullable<Int8>,
        owner -> Nullable<Varchar>,
    }
}

table! {
    solana_inst_initialize_nonces (id) {
        id -> Int8,
        tx_hash -> Nullable<Varchar>,
        block_time -> Nullable<Int8>,
        inst_order -> Nullable<Int4>,
        nonce_account -> Nullable<Varchar>,
        recent_block_hashes_sysvar -> Nullable<Varchar>,
        rent_sysvar -> Nullable<Varchar>,
        nonce_authority -> Nullable<Varchar>,
    }
}

table! {
    solana_inst_transfers (id) {
        id -> Int8,
        tx_hash -> Nullable<Varchar>,
        block_time -> Nullable<Int8>,
        inst_order -> Nullable<Int4>,
        source -> Nullable<Varchar>,
        destination -> Nullable<Varchar>,
        lamports -> Nullable<Int8>,
        source_base -> Nullable<Varchar>,
        source_seed -> Nullable<Text>,
        source_owner -> Nullable<Varchar>,
    }
}

table! {
    solana_inst_withdraw_from_nonces (id) {
        id -> Int8,
        tx_hash -> Nullable<Varchar>,
        block_time -> Nullable<Int8>,
        inst_order -> Nullable<Int4>,
        nonce_account -> Nullable<Varchar>,
        destination -> Nullable<Varchar>,
        recent_block_hashes_sysvar -> Nullable<Varchar>,
        rent_sysvar -> Nullable<Varchar>,
        nonce_authority -> Nullable<Varchar>,
        lamports -> Nullable<Int8>,
    }
}

table! {
    solana_instructions (id) {
        id -> Int8,
        block_slot -> Nullable<Int8>,
        tx_index -> Nullable<Int2>,
        block_time -> Nullable<Int8>,
        inst_index -> Nullable<Int4>,
        program_name -> Nullable<Text>,
        accounts -> Nullable<Array<Text>>,
        data -> Nullable<Bytea>,
    }
}

table! {
    solana_logs (tx_hash) {
        tx_hash -> Varchar,
        log_messages -> Nullable<Array<Text>>,
        block_time -> Nullable<Int8>,
    }
}

table! {
    solana_token_balances (id) {
        id -> Int8,
        block_slot -> Nullable<Int8>,
        tx_index -> Nullable<Int2>,
        account -> Nullable<Varchar>,
        token_address -> Nullable<Varchar>,
        decimals -> Nullable<Int2>,
        pre_amount -> Nullable<Int8>,
        post_amount -> Nullable<Int8>,
    }
}

table! {
    solana_transactions (signatures) {
        block_slot -> Nullable<Int8>,
        tx_index -> Nullable<Int2>,
        signatures -> Varchar,
        signers -> Nullable<Text>,
        reward -> Nullable<Int8>,
        fee -> Nullable<Int8>,
        status -> Nullable<Bpchar>,
    }
}

allow_tables_to_appear_in_same_query!(
    ethereum_blocks,
    ethereum_daily_address_transactions,
    ethereum_daily_transactions,
    ethereum_transactions,
    network_states,
    solana_account_transactions,
    solana_blocks,
    solana_daily_stat_blocks,
    solana_inner_instructions,
    solana_inst_advance_nonces,
    solana_inst_allocates,
    solana_inst_assigns,
    solana_inst_authorize_nonces,
    solana_inst_create_accounts,
    solana_inst_initialize_nonces,
    solana_inst_transfers,
    solana_inst_withdraw_from_nonces,
    solana_instructions,
    solana_logs,
    solana_token_balances,
    solana_transactions,
);
