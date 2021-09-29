-- alter table ethereum_daily_transaction drop column timestamp;
-- alter table ethereum_daily_transaction alter column transaction_date type date using transaction_date::date;
alter table ethereum_block drop column parent_hash;