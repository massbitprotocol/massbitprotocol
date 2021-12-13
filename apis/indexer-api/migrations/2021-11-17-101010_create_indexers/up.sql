CREATE TYPE indexer_status AS ENUM ('draft', 'deploying', 'deployed', 'stopped','invalid');
create table indexers
(
    network varchar,    -- network name: ex: mainnet
    name varchar not null, -- Indexer name from manifest
    namespace varchar not null, -- schema name
    description varchar,
    image_url varchar,
    repository varchar,               -- public Github repo url
    manifest varchar not null,  -- hash of manifest file from IPFS
    mapping varchar not null,   -- hash of mapping file from IPFS
    graphql varchar not null,   -- hash of graphql file from IPFS
    status indexer_status not null,
    deleted bool not null default false, -- logical deleted indexer
    address varchar, -- interested address of indexer
    start_block bigint default 0 not null, -- start block from manifest
    got_block bigint default 0 not null,    --last got block
    version varchar,
    hash varchar not null,
    v_id bigserial
        constraint indexers_pk
            primary key
);

create unique index indexers_hash_uindex
    on indexers (hash);

