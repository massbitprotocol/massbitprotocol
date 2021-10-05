create table solana_logs
(
    tx_hash         varchar constraint solana_logs_pk primary key,
    log_messages    text[],
    block_time      bigint
);

create table solana_instructions
(
    id              bigserial constraint solana_instructions_pk primary key,
    block_hash      varchar(100),
    tx_hash         varchar(100),
    block_time      bigint,
    inst_order      int,
    program_name    text,
    accounts        text[],
    data            bytea,
    encoded_data    text
);

create table solana_inner_instructions
(
    id              bigserial constraint solana_inner_instructions_pk primary key

);
