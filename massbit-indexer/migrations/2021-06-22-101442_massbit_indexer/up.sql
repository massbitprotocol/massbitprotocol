CREATE TABLE solana_block (
                              block_number BIGINT PRIMARY KEY,
                              timestamp BIGINT NOT NULL,
                              transaction_number BIGINT NOT NULL,
                              sol_transfer BIGINT NOT NULL,
                              fee BIGINT NOT NULL
);

CREATE TABLE solana_address (
                                id BIGSERIAL PRIMARY KEY,
                                block_number BIGINT NOT NULL,
                                timestamp BIGINT NOT NULL,
                                address TEXT NOT NULL,
                                is_new_create BOOLEAN NOT NULL,
                                balance BIGINT NOT NULL
);
