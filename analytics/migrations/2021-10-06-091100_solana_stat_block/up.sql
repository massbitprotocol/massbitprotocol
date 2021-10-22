create table solana_daily_stat_blocks
(
    id                      bigserial constraint solana_daily_stat_blocks_pk primary key,
    network                 varchar(100),
    date                    bigint,
    min_block_slot          bigint,
    max_block_slot          bigint,
    block_counter           bigint,     -- Number of indexed blocks
    total_tx                bigint,
    success_tx              bigint,
    total_reward            bigint,
    total_fee               bigint,
    average_block_time      bigint default 0,   -- average time  generated block in ms
    fist_block_time         bigint,
    last_block_time         bigint,
    constraint solana_daily_stat_blocks_date_uindex
        unique (network, date)
)