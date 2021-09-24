create table ethereum_block
(
    block_hash          text constraint ethereum_block_pk primary key,
    block_number        bigint,
    transaction_number  bigint,
    timestamp           bigint not null,
    validated_by        text,
    reward              numeric,
    difficulty          numeric,
    total_difficulty    numeric,
    size                bigint,
    gas_used            numeric,
    gas_limit           numeric,
    extra_data          bytea
);

create table ethereum_transaction
(
    transaction_hash    text primary key ,
    block_hash          text,
    block_number        bigint,
    nonce               numeric,
    sender              text      not null,
    receiver            text,
    value               numeric   not null,
    gas_limit           numeric   not null,
    gas_price           numeric   not null,
    timestamp           bigint   not null
);

create index attr_0_1_ethereum_transaction_block_hash
    on ethereum_transaction ("left"(block_hash, 256));

create index attr_0_2_ethereum_transaction_block_number
    on ethereum_transaction (block_number);

create index attr_0_3_ethereum_transaction_nonce
    on ethereum_transaction (nonce);

create index attr_0_4_ethereum_transaction_sender
    on ethereum_transaction ("left"(sender, 256));

create index attr_0_5_ethereum_transaction_receiver
    on ethereum_transaction ("left"(receiver, 256));

create index attr_0_6_ethereum_transaction_value
    on ethereum_transaction (value);

create index attr_0_7_ethereum_transaction_gas_limit
    on ethereum_transaction (gas_limit);

create index attr_0_8_ethereum_transaction_gas_price
    on ethereum_transaction (gas_price);

create index attr_0_10_ethereum_transaction_timestamp
    on ethereum_transaction (timestamp);

create table ethereum_daily_transaction
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
    constraint ethereum_daily_transaction_transaction_date_network_uindex
        unique (transaction_date, network)
);

create table ethereum_daily_address_transaction
(
    id integer generated always as identity
        constraint daily_address_transaction_pkey
            primary key,
    address text,
    transaction_date date not null,
    transaction_count numeric not null,
    transaction_volume numeric not null,
    gas numeric not null,
    constraint ethereum_daily_address_transaction_date_uindex
        unique (address, transaction_date)
);


CREATE OR REPLACE FUNCTION insert_ethereum_transaction()
  RETURNS TRIGGER
  LANGUAGE PLPGSQL
  AS
$$
BEGIN
INSERT INTO ethereum_daily_transaction(network, transaction_date, transaction_count, transaction_volume, gas, average_gas_price)
VALUES('matic', to_timestamp(NEW.timestamp)::date, 1, NEW.value, NEW.gas_limit, NEW.gas_price)
    ON CONFLICT (transaction_date, network) DO
UPDATE SET transaction_count = ethereum_daily_transaction.transaction_count + EXCLUDED.transaction_count,
    transaction_volume = ethereum_daily_transaction.transaction_volume + EXCLUDED.transaction_volume,
    gas = ethereum_daily_transaction.gas + EXCLUDED.gas,
    average_gas_price = (ethereum_daily_transaction.average_gas_price * ethereum_daily_transaction.transaction_count + EXCLUDED.average_gas_price)
    / (ethereum_daily_transaction.transaction_count + 1);
INSERT INTO ethereum_daily_address_transaction(address, transaction_date, transaction_count, transaction_volume, gas)
VALUES(NEW.sender, to_timestamp(NEW.timestamp)::date, 1, NEW.value, NEW.gas_limit)
    ON CONFLICT (address, transaction_date) DO
UPDATE SET transaction_count = ethereum_daily_address_transaction.transaction_count + EXCLUDED.transaction_count,
    transaction_volume = ethereum_daily_address_transaction.transaction_volume + EXCLUDED.transaction_volume,
    gas = ethereum_daily_address_transaction.gas + EXCLUDED.gas;
RETURN NEW;
END;
$$;

create trigger insert_ethereum_transaction
    after insert
    on ethereum_transaction
    for each row
    execute procedure insert_ethereum_transaction();