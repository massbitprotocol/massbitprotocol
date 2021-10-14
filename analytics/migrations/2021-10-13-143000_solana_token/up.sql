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
    signers             text,
    authority           varchar(88),
    multisig_authority  varchar(88)
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
    signers             text,
    owner               varchar(88),
    multisig_owner      text
);