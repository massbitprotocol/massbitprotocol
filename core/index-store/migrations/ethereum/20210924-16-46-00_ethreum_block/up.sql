create table ethereum_block
(
    hash varchar not null
        constraint ethereum_block_pkey
            primary key,
    number bigint not null,
    parent_hash varchar not null,
    network_name varchar not null,
    data jsonb not null
);

create index ethereum_block_name_number
    on ethereum_block (network_name, number);

