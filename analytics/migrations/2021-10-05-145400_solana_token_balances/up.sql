create table solana_token_balances
(
    id              bigserial constraint solana_token_balances_pk primary key,
    tx_hash         varchar(100),
    account         varchar(100),
    token_address   varchar(100),
    decimals        int,
    pre_amount      bigint,
    post_amount     bigint


)