select owner_account, source, amount_in, source_mint_account, to_timestamp(floor(block_time/86400) * 86400)::date as date  from sgd18.saber_swap;
select owner_account, source, minimum_amount_out, destination_mint_account, to_timestamp(floor(block_time/86400) * 86400)::date as date  from sgd18.saber_swap;

select owner_account, source, sum(amount_in), source_mint_account, to_timestamp(floor(block_time/86400) * 86400)::date as date
from sgd18.saber_swap group by owner_account, source, source_mint_account, date;
select owner_account, source, sum(minimum_amount_out), destination_mint_account, to_timestamp(floor(block_time/86400) * 86400)::date as date
from sgd18.saber_swap group by owner_account, source, destination_mint_account, date;

select owner_account, token_a, token_a_amount, token_a_mint_account, token_b, token_b_amount, token_b_mint_account, to_timestamp(floor(block_time/86400) * 86400)::date as date from sgd18.saber_deposit;
