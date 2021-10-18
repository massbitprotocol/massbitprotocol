create table solana_blocks
(
    block_slot       bigint,
    block_hash       varchar(100) constraint solana_blocks_pk primary key,
    previous_block_hash      varchar(100),
    parent_slot      bigint,
    transaction_number bigint,
    timestamp       bigint,
    leader          varchar(100) default '',
    reward          bigint default 0
);
create index solana_blocks_block_height_index
    on solana_blocks (block_slot );

create table solana_transactions
(
    block_slot          bigint,
    tx_index            smallint,
    signatures          varchar(100),
    signers             text,
    reward              bigint default 0,
    fee                 bigint,
    status              char(1)
);
create index solana_transactions_block_height_order_in_block_index
    on solana_transactions (block_slot, tx_index);

create table solana_account_transactions
(
    block_slot          bigint,
    tx_index            smallint,
    account             varchar(100),
    pre_balance         bigint,
    post_balance        bigint
);

create index solana_account_transactions_block_height_order_in_block_index
    on solana_account_transactions (block_slot, tx_index);