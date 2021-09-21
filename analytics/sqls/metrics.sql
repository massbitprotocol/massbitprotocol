-- Trading volume
SELECT transaction_date, transaction_volume FROM matic_daily_transaction;
-- Transaction volume
SELECT transaction_date, transaction_count FROM matic_daily_transaction;
-- Active addresses
SELECT COUNT(address), transaction_date FROM matic_daily_address_transaction GROUP BY transaction_date;
-- Total used gas
SELECT transaction_date, gas FROM matic_daily_transaction;
-- Average Gas Price
SELECT transaction_date, average_gas_price FROM matic_daily_transaction;