-- Trading volume
SELECT transaction_date, transaction_volume FROM ethereum_daily_transaction;
-- Transaction volume
SELECT transaction_date, transaction_count FROM ethereum_daily_transaction;
-- Active addresses
SELECT COUNT(address) as active_address, transaction_date FROM ethereum_daily_address_transaction GROUP BY transaction_date;
-- Total used gas
SELECT transaction_date, gas FROM ethereum_daily_transaction;
-- Average Gas Price
SELECT transaction_date, average_gas_price FROM ethereum_daily_transaction;