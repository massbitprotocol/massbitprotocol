create table network_states
(
    id              bigserial constraint network_states_pk primary key,
    chain           text not null,
    network         text not null default '',
    got_block       bigint not null default 0
);
create unique index network_states_chain_network_uindex
    on network_states (chain, network);

