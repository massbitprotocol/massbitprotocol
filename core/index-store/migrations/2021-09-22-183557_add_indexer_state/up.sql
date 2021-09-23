create table indexer_state
(
    id serial
        constraint indexer_state_pk
            primary key,
    indexer_hash varchar not null,
    schema_name varchar not null,
    got_block bigint default 0 not null
);


