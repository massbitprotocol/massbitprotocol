alter table ethereum_blocks add parent_hash varchar;
alter table ethereum_transactions rename column gas_limit to gas;
drop trigger insert_ethereum_transaction on ethereum_transactions;
alter table ethereum_daily_transactions alter column transaction_date type varchar using transaction_date::varchar;
alter table ethereum_daily_transactions add timestamp bigint;
alter table ethereum_daily_address_transactions alter column transaction_date type varchar using transaction_date::varchar;
alter table ethereum_daily_address_transactions add timestamp bigint;