create table indexer_deployment_schemas
(
    id           serial
        primary key,
    created_at   timestamp with time zone                     not null,
    indexer_hash varchar                                      not null,
    schema_name  varchar                                      not null,
    shard        varchar default 'primary'::character varying not null,
    network      varchar default ''::character varying        not null,
    active       boolean default true                         not null
);

CREATE TYPE indexer_health AS ENUM ('health', 'failed', 'unhealth');
create table indexer_deployments
(
    id                        integer not null
        primary key,
    hash                      varchar,
    namespace                 varchar not null,
    schema                    varchar not null,
    failed                    boolean not null,
    health                    indexer_health,
    synced                    boolean not null,
    fatal_error               text,
    non_fatal_errors          text[] default '{}'::text[],
    earliest_block_hash       bytea,
    earliest_block_number     numeric,
    latest_block_hash         bytea,
    latest_block_number       numeric,
    last_healthy_block_hash   bytea,
    last_healthy_block_number numeric,
    entity_count              numeric,
    graft_base                text,
    graft_block_hash          bytea,
    graft_block_number        numeric,
    reorg_count               integer,
    current_reorg_depth       integer,
    max_reorg_depth           integer
);
