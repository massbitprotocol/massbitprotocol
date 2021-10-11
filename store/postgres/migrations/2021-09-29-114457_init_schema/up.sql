-- Your SQL goes here
-- This requires superuser privileges
create extension if not exists btree_gist;

create table deployment_schemas
(
    id         serial
        constraint deployment_schemas_pkey
            primary key,
    indexer    varchar                                                                                          not null,
    name       varchar                  default ('sgd'::text || currval('deployment_schemas_id_seq'::regclass)) not null,
    shard      text                                                                                             not null,
    network    text                                                                                             not null,
    created_at timestamp with time zone default now()                                                           not null
);

create table dynamic_ethereum_contract_data_source
(
    vid                   bigserial
        constraint dynamic_ethereum_contract_data_source_pkey
            primary key,
    name                  text    not null,
    ethereum_block_hash   bytea   not null,
    ethereum_block_number numeric not null,
    deployment            text    not null,
    context               text,
    address               bytea   not null,
    abi                   text    not null,
    start_block           integer not null
);

create table indexer_manifest
(
    id           integer                     not null
        constraint indexer_manifest_pkey
            primary key,
    spec_version text                        not null,
    description  text,
    repository   text,
    schema       text                        not null,
    features     text[] default '{}'::text[] not null
);

create table indexer
(
    id              text      not null,
    name            text      not null
        constraint indexer_name_uq
            unique,
    created_at      numeric   not null,
    vid             bigserial
        constraint s_pkey
            primary key,
    block_range     int4range not null,
    constraint indexer_id_block_range_excl
        exclude using gist (id with pg_catalog.=, block_range with pg_catalog.&&)
);

create table indexer_deployment
(
    id                                 integer           not null
        constraint indexer_deployment_pkey
            primary key,
    deployment                         text              not null
        constraint indexer_deployment_id_key
            unique,
    earliest_ethereum_block_hash       bytea,
    earliest_ethereum_block_number     numeric,
    latest_ethereum_block_hash         bytea,
    latest_ethereum_block_number       numeric,
    entity_count                       numeric           not null,
    last_healthy_ethereum_block_hash   bytea,
    last_healthy_ethereum_block_number numeric
);

create table table_stats
(
    id              serial
        constraint table_stats_pkey
            primary key,
    deployment      integer not null
        constraint table_stats_deployment_fkey
            references indexer_deployment
            on delete cascade,
    table_name      text    not null,
    is_account_like boolean,
    constraint table_stats_deployment_table_name_key
        unique (deployment, table_name)
);
