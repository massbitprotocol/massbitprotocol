CREATE OR REPLACE FUNCTION insert_matic_transaction()
  RETURNS TRIGGER
  LANGUAGE PLPGSQL
  AS
$$
BEGIN
    INSERT INTO sgd0.daily_transaction(network, transaction_date, transaction_count, transaction_volume, gas, average_gas_price)
        VALUES('matic', to_timestamp(NEW.timestamp)::date, 1, NEW.value, NEW.gas, NEW.gas_price)
    ON CONFLICT (transaction_date, network) DO
    UPDATE SET transaction_count = sgd0.daily_transaction.transaction_count + EXCLUDED.transaction_count,
               transaction_volume = sgd0.daily_transaction.transaction_volume + EXCLUDED.transaction_volume,
               gas = sgd0.daily_transaction.gas + EXCLUDED.gas,
               average_gas_price = (sgd0.daily_transaction.average_gas_price * sgd0.daily_transaction.transaction_count + EXCLUDED.average_gas_price)
                    / (sgd0.daily_transaction.transaction_count + 1);
    INSERT INTO sgd0.daily_matic_address_transaction(address, transaction_date, transaction_count, transaction_volume, gas)
    VALUES(NEW.sender, to_timestamp(NEW.timestamp)::date, 1, NEW.value, NEW.gas)
    ON CONFLICT (address, transaction_date) DO
        UPDATE SET transaction_count = sgd0.daily_matic_address_transaction.transaction_count + EXCLUDED.transaction_count,
                   transaction_volume = sgd0.daily_matic_address_transaction.transaction_volume + EXCLUDED.transaction_volume,
                   gas = sgd0.daily_matic_address_transaction.gas + EXCLUDED.gas;
RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS  insert_matic_transaction
    ON sgd0.matic_transaction;
CREATE TRIGGER insert_matic_transaction
    AFTER INSERT
    ON sgd0.matic_transaction
    FOR EACH ROW
    EXECUTE PROCEDURE insert_matic_transaction();