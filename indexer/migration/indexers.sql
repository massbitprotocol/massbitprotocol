CREATE TABLE IF NOT EXISTS indexers
(
    id varchar,
    network varchar,
    name varchar,
    description varchar,
    repo varchar,
    index_status varchar,
    index_data_ref varchar,
    v_id serial
    constraint indexers_pk
    primary key
);
