create table indexer_deployment_schemas
(
    id serial primary key,
    created_at timestamptz not null,
    indexer_hash varchar not null,
    schema_name varchar not null,
    shard varchar not null default 'primary',
    network varchar not null default '',
    active bool not null default true
);