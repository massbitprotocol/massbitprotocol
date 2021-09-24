create table network_state
(
    id              bigserial constraint id_pk primary key,
    chain           text not null,
    network         text not null default '',
    got_block       bigint not null default 0
);
create unique index network_state_chain_network_uindex
    on network_state (chain, network);

