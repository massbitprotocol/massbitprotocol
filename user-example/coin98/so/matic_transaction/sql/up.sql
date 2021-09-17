drop table if exists daily_transaction;
create table daily_transaction
(
    id integer generated always as identity
        constraint daily_transaction_pkey
            primary key,
    network varchar(40) not null,
    transaction_date date not null,
    transaction_count numeric not null,
    transaction_volume numeric not null,
    gas numeric not null,
    average_gas_price numeric not null default 0,
    constraint daily_transaction_transaction_date_network_uindex
        unique (transaction_date, network)
);

drop table if exists daily_matic_address_transaction;
create table daily_matic_address_transaction
(
    id integer generated always as identity
        constraint daily_address_transaction_pkey
            primary key,
    address text,
    transaction_date date not null,
    transaction_count numeric not null,
    transaction_volume numeric not null,
    gas numeric not null,
    constraint daily_matic_address_transaction_date_uindex
        unique (address, transaction_date)
);
