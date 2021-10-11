create table solana_inst_create_accounts
(
    id              bigserial constraint solana_inst_create_accounts_pk primary key,
    tx_hash         varchar(100),
    block_time      bigint,
    inst_order      int,        --instruction order in transaction
    source          varchar(100),
    new_account     varchar(100),
    base            varchar(100),
    seed            text,
    lamports        bigint,
    space           bigint,
    owner           varchar(100)
);

create table solana_inst_assigns
(
    id              bigserial constraint solana_inst_assigns_pk primary key,
    tx_hash         varchar(100),
    block_time      bigint,
    inst_order      int,        --instruction order in transaction
    account         varchar(100),
    base            varchar(100),
    seed            text,
    owner           varchar(100)
);

create table solana_inst_transfers
(
    id              bigserial constraint solana_inst_transfers_pk primary key,
    tx_hash         varchar(100),
    block_time      bigint,
    inst_order      int,        --instruction order in transaction
    source          varchar(100),
    destination     varchar(100),
    lamports        bigint,
    -- with seed
    source_base     varchar(100),
    source_seed     text,
    source_owner    varchar(100)
);



create table solana_inst_advance_nonces
(
    id                      bigserial constraint solana_inst_advance_nonces_pk primary key,
    tx_hash                 varchar(100),
    block_time              bigint,
    inst_order              int,        --instruction order in transaction
    nonce_account           varchar(100),
    recent_block_hashes_sysvar varchar(100),
    nonce_authority         varchar(100)
);

create table solana_inst_withdraw_from_nonces
(
    id                      bigserial constraint solana_inst_withdraw_from_nonces_pk primary key,
    tx_hash                 varchar(100),
    block_time              bigint,
    inst_order              int,        --instruction order in transaction
    nonce_account           varchar(100),
    destination             varchar(100),
    recent_block_hashes_sysvar varchar(100),
    rent_sysvar             varchar(100),
    nonce_authority         varchar(100),
    lamports                bigint
);

create table solana_inst_initialize_nonces
(
    id                      bigserial constraint solana_inst_initialize_nonces_pk primary key,
    tx_hash                 varchar(100),
    block_time              bigint,
    inst_order              int,        --instruction order in transaction
    nonce_account           varchar(100),
    recent_block_hashes_sysvar varchar(100),
    rent_sysvar             varchar(100),
    nonce_authority         varchar(100)
);

create table solana_inst_authorize_nonces
(
    id                      bigserial constraint solana_inst_authorize_nonces_pk primary key,
    tx_hash                 varchar(100),
    block_time              bigint,
    inst_order              int,        --instruction order in transaction
    nonce_account           varchar(100),
    nonce_authority         varchar(100),
    new_authorized          varchar(100)
);

create table solana_inst_allocates
(
    id                      bigserial constraint solana_inst_allocates_pk primary key,
    tx_hash                 varchar(100),
    block_time              bigint,
    inst_order              int,        --instruction order in transaction
    account                 varchar(100),
    space                   bigint,
    --with seeds
    base                    varchar(100),
    seed                    text,
    owner                   varchar(100)
);
