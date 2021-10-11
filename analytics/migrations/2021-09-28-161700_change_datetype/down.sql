alter table ethereum_block drop column parent_hash;
alter table ethereum_transaction rename column gas to gas_limit;
alter table ethereum_daily_transaction drop column timestamp;
alter table ethereum_daily_transaction alter column transaction_date type date using transaction_date::date;
alter table ethereum_daily_address_transaction drop column timestamp;
alter table ethereum_daily_address_transaction alter column transaction_date type date using transaction_date::date;