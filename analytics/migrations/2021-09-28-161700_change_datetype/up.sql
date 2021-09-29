alter table ethereum_block add parent_hash varchar;
alter table ethereum_transaction rename column gas_limit to gas;
drop trigger insert_ethereum_transaction on ethereum_transaction;
alter table ethereum_daily_transaction alter column transaction_date type varchar using transaction_date::varchar;
alter table ethereum_daily_transaction add timestamp bigint;
alter table ethereum_daily_address_transaction alter column transaction_date type varchar using transaction_date::varchar;
alter table ethereum_daily_address_transaction add timestamp bigint;