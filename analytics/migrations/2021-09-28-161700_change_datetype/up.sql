-- alter table ethereum_daily_transaction alter column transaction_date type varchar using transaction_date::varchar;
-- alter table ethereum_daily_transaction add timestamp bigint;