create table solana_daily_stat_block
(
    id                      bigserial constraint solana_daily_stat_block_pk primary key,
    network                 varchar(100),
    date                    bigint,
    min_block_height        bigint,
    max_block_height        bigint,
    transaction_counter     bigint,
    average_reward          bigint,
    fist_block_time         bigint,
    last_block_time         bigint,
    average_block_time      bigint default 0,   -- average time  generated block in ms
    constraint solana_daily_stat_block_date_uindex
        unique (network, date)
)