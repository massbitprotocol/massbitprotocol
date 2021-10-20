create table solana_token_balances
(
    id              bigserial constraint solana_token_balances_pk primary key,
    block_slot      bigint,
    tx_index        smallint,
    account         varchar(100),
    token_address   varchar(100),
    decimals        smallint,
    pre_amount      bigint,
    post_amount     bigint


)