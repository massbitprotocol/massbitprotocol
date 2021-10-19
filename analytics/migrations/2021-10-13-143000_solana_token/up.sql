create table solana_spl_token_initialize_mint
(
    id                  bigserial constraint solana_spl_token_initialize_mint_pk primary key,
    block_slot          bigint,
    block_time          bigint,
    tx_index            int,    --Index of transaction in block
    instruction_index   int,    --Index of instruction in transaction
    mint                varchar(88),
    decimals            smallint,
    mint_authority      varchar(88),
    rent_sysvar         varchar(88),
    freeze_authority    varchar(88)
);

create table solana_spl_token_initialize_account
(
    id                  bigserial constraint solana_spl_token_initialize_account_pk primary key,
    block_slot          bigint,
    block_time          bigint,
    tx_index            int,    --Index of transaction in block
    instruction_index   int,    --Index of instruction in transaction
    account             varchar(88),
    mint                varchar(88),
    owner               varchar(88),
    rent_sysvar         varchar(88)
);

create table solana_spl_token_initialize_account2
(
    id                  bigserial constraint solana_spl_token_initialize_account2_pk primary key,
    block_slot          bigint,
    block_time          bigint,
    tx_index            int,    --Index of transaction in block
    instruction_index   int,    --Index of instruction in transaction
    account             varchar(88),
    mint                varchar(88),
    owner               varchar(88),
    rent_sysvar         varchar(88)
);

create table solana_spl_token_initialize_multisig
(
    id                  bigserial constraint solana_spl_token_initialize_multisig_pk primary key,
    block_slot          bigint,
    block_time          bigint,
    tx_index            int,    --Index of transaction in block
    instruction_index   int,    --Index of instruction in transaction
    multisig            varchar(88),
    rent_sysvar         varchar(88),
    signers             text,
    m                   smallint

);

create table solana_spl_token_transfer
(
    id                  bigserial constraint solana_spl_token_transfer_pk primary key,
    block_slot          bigint,
    block_time          bigint,
    tx_index            int,    --Index of transaction in block
    instruction_index   int,    --Index of instruction in transaction
    source              varchar(88),
    destination         varchar(88),
    amount              bigint,
    authority           varchar(88),
    multisig_authority  varchar(88),
    signers             text
);
create table solana_spl_token_transfer_checked
(
    id                          bigserial constraint solana_spl_token_transfer_checked_pk primary key,
    block_slot                  bigint,
    block_time                  bigint,
    tx_index                    int,    --Index of transaction in block
    instruction_index           int,    --Index of instruction in transaction
    source                      varchar(88),
    mint                        varchar(88),
    destination                 varchar(88),
    token_amount                varchar(88),
    authority                   varchar(88),
    multisig_fauthority         text,
    signers                     text
);
create table solana_spl_token_approve
(
    id                  bigserial constraint solana_spl_token_approve_pk primary key,
    block_slot          bigint,
    block_time          bigint,
    tx_index            int,    --Index of transaction in block
    instruction_index   int,    --Index of instruction in transaction
    source              varchar(88),
    delegate            varchar(88),
    amount              bigint,
    owner               varchar(88),
    multisig_owner      text,
    signers             text
);


create table solana_spl_token_approve_checked
(
    id                          bigserial constraint solana_spl_token_approve_checked_pk primary key,
    block_slot                  bigint,
    block_time                  bigint,
    tx_index                    int,    --Index of transaction in block
    instruction_index           int,    --Index of instruction in transaction
    source                      varchar(88),
    mint                        varchar(88),
    delegate                    varchar(88),
    token_amount                varchar(88),
    owner                       varchar(88),
    multisig_owner              text,
    signers                     text
);

create table solana_spl_token_revoke
(
    id                  bigserial constraint solana_spl_token_revoke_pk primary key,
    block_slot          bigint,
    block_time          bigint,
    tx_index            int,    --Index of transaction in block
    instruction_index   int,    --Index of instruction in transaction
    source              varchar(88),
    signers             text,
    owner               varchar(88),
    multisig_owner      text
);

create table solana_spl_token_set_authority
(
    id                  bigserial constraint solana_spl_token_set_authority_pk primary key,
    block_slot          bigint,
    block_time          bigint,
    tx_index            int,    --Index of transaction in block
    instruction_index   int,    --Index of instruction in transaction
    authority_type      varchar(88),
    new_authority       varchar(88),
    signers             text,
    authority           varchar(88),
    multisig_authority  text
);

create table solana_spl_token_mint_to
(
    id                  bigserial constraint solana_spl_token_mint_to_pk primary key,
    block_slot          bigint,
    block_time          bigint,
    tx_index            int,    --Index of transaction in block
    instruction_index   int,    --Index of instruction in transaction
    mint                varchar(88),
    account             varchar(88),
    amount              varchar(88),
    mint_authority      varchar(88),
    multisig_mint_authority  text,
    signers             text
);

create table solana_spl_token_min_to_checked
(
    id                          bigserial constraint solana_spl_token_min_to_checked_pk primary key,
    block_slot                  bigint,
    block_time                  bigint,
    tx_index                    int,    --Index of transaction in block
    instruction_index           int,    --Index of instruction in transaction
    mint                        varchar(88),
    account                     varchar(88),
    token_amount                varchar(88),
    mint_authority              varchar(88),
    multisig_mint_authority     varchar(88),
    signers                     text
);

create table solana_spl_token_burn
(
    id                  bigserial constraint solana_spl_token_burn_pk primary key,
    block_slot          bigint,
    block_time          bigint,
    tx_index            int,    --Index of transaction in block
    instruction_index   int,    --Index of instruction in transaction
    account             varchar(88),
    mint                varchar(88),
    amount              varchar(88),
    signers             text,
    authority      varchar(88),
    multisig_authority  text
);

create table solana_spl_token_burn_checked
(
    id                          bigserial constraint solana_spl_token_burn_checked_pk primary key,
    block_slot                  bigint,
    block_time                  bigint,
    tx_index                    int,    --Index of transaction in block
    instruction_index           int,    --Index of instruction in transaction
    account                     varchar(88),
    mint                        varchar(88),
    token_amount                varchar(88),
    authority              varchar(88),
    multisig_mint_authority     varchar(88),
    signers                     text
);

create table solana_spl_token_close_account
(
    id                  bigserial constraint solana_spl_token_close_account_pk primary key,
    block_slot          bigint,
    block_time          bigint,
    tx_index            int,    --Index of transaction in block
    instruction_index   int,    --Index of instruction in transaction
    account             varchar(88),
    destination         varchar(88),
    owner               varchar(88),
    signers             text,
    multisig_owner      text
);

create table solana_spl_token_freeze_account
(
    id                          bigserial constraint solana_spl_token_freeze_account_pk primary key,
    block_slot                  bigint,
    block_time                  bigint,
    tx_index                    int,    --Index of transaction in block
    instruction_index           int,    --Index of instruction in transaction
    account                     varchar(88),
    mint                        varchar(88),
    freeze_authority            varchar(88),
    signers                     text,
    multisig_freeze_authority   text
);

create table solana_spl_token_thaw_account
(
    id                          bigserial constraint solana_spl_token_thaw_account_pk primary key,
    block_slot                  bigint,
    block_time                  bigint,
    tx_index                    int,    --Index of transaction in block
    instruction_index           int,    --Index of instruction in transaction
    account                     varchar(88),
    mint                        varchar(88),
    freeze_authority            varchar(88),
    signers                     text,
    multisig_freeze_authority   text
);

create table solana_spl_token_sync_native
(
    id                          bigserial constraint solana_spl_token_sync_native_pk primary key,
    block_slot                  bigint,
    block_time                  bigint,
    tx_index                    int,    --Index of transaction in block
    instruction_index           int,    --Index of instruction in transaction
    account                     varchar(88)
);
