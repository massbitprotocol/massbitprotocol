create table solana_spl_token_initialize_mint
(
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
    block_slot                  bigint,
    block_time                  bigint,
    tx_index                    int,    --Index of transaction in block
    instruction_index           int,    --Index of instruction in transaction
    account                     varchar(88)
);
