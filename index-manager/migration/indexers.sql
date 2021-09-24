CREATE TABLE IF NOT EXISTS indexers
(
    id varchar,
    network varchar,
    name varchar,
    schema_name varchar not null,
    description varchar,
    repo varchar,
    index_status varchar,
    got_block bigint default 0 not null,
    hash varchar,
    v_id serial
    constraint indexers_pk
    primary key
);
