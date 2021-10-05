create table solana_blocks
(
    block_hash       varchar(100) constraint solana_blocks_pk primary key,
    previous_block_hash      varchar(100),
    parent_slot      bigint,
    block_height     bigint,
    transaction_number bigint,
    timestamp       bigint,
    leader          varchar(100) default '',
    reward          bigint default 0
);

create table solana_transactions
(
    id                  bigserial constraint solana_transactions_pk primary key,
    block_hash          varchar(100),
    block_number        bigint,
    parent_slot         bigint,
    signatures          text,
    signers             text,
    block_time          bigint,
    reward              bigint default 0,
    fee                 bigint,
    status              varchar(10)
);

create table solana_account_transactions
(
    id                  bigserial constraint solana_account_transactions_pk primary key,
    account             varchar(100),
    tx_hash             varchar(100),
    pre_balance         bigint,
    post_balance        bigint
)
