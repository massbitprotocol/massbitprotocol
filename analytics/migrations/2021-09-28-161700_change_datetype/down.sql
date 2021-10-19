alter table ethereum_blocks drop column parent_hash;
alter table ethereum_transactions rename column gas to gas_limit;
alter table ethereum_daily_transactions drop column timestamp;
alter table ethereum_daily_transactions alter column transaction_date type date using transaction_date::date;
alter table ethereum_daily_address_transactions drop column timestamp;
alter table ethereum_daily_address_transactions alter column transaction_date type date using transaction_date::date;