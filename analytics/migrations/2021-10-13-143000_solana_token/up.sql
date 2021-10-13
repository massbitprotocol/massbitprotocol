create table solana_tokens
(
    account             varchar(88),
    owner               varchar(88),
    name                text,
    symbol              varchar(20),
    website             varchar(100),
    authority           varchar(88),
    max_total_supply    bigint,
    decimals            int
);