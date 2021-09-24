CREATE TABLE IF NOT EXISTS indexers
(
    id varchar,
    network varchar,
    name varchar not null,
    namespace varchar not null,
    description varchar,
    repo varchar,
    manifest varchar not null,
    index_status varchar,
    got_block bigint default 0 not null,
    hash varchar,
    v_id serial
        constraint indexers_pk
            primary key
);
create unique index indexers_hash_uindex
    on indexers (hash);