create table solana_blocks
(
    block_hash       varchar(50) constraint solana_blocks_pk primary key,
    previous_block_hash      varchar(50),
    parent_slot      bigint,
    block_height     bigint,
    transaction_number bigint,
    timestamp       bigint,
    leader          varchar(50) default '',
    reward          bigint default 0
);

create table solana_transactions
(
    id                  bigserial constraint solana_transactions_pk primary key,
    block_hash          varchar(50),
    block_number        bigint,
    signatures          text,
    signers             text,
    timestamp       bigint,
    reward          bigint default 0
);
