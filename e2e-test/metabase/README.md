# Queries for Solana and it's Defi
## Saber
Deposit order by Token pair
```postgresql
select count(*) counter, token_a_mint_account, token_b_mint_account
from sgd2.saber_deposit where floor(block_time/86400)  = floor(extract(epoch from now()) / 86400) - 1 
group by token_a_mint_account, token_b_mint_account order by counter desc limit 10;
```

Deposits of the most active user last day
```postgresql
select d.owner_account, token_a_amount, token_a_mint_account, token_b_amount, token_b_mint_account, min_mint_amount
from sgd2.saber_deposit as d, (select count(*) as counter , owner_account from sgd2.saber_deposit group by owner_account order by counter desc limit 1) as t
where d.owner_account = t.owner_account and floor(block_time/86400) = floor(extract(epoch from now()) / 86400) - 1
order by block_time;
```

Swap amount by token in a day
```postgresql
select sum(amount_in), source_mint_account
from sgd2.saber_swap where floor(block_time/86400) * 86400 = 1634169600 group by  source_mint_account;
```

Swap counter by owner
```postgresql
select count(amount_in) as amount, owner_account
from sgd2.saber_swap
where floor(block_time/86400) = floor(extract(epoch from now()) / 86400) - 1 group by  owner_account order by amount desc limit 10; 
```

Swap orders by owner
```postgresql
select count(amount_in) as amount, owner_account
from sgd2.saber_swap
where floor(block_time/86400) = floor(extract(epoch from now()) / 86400) - 1 group by  owner_account order by amount desc limit 10; 
```

## Solana data
Daily average reward and fee
```postgresql
select solana_daily_stat_block.total_tx, solana_daily_stat_block.success_tx, total_reward/block_counter as average_reward, average_block_time,
       total_fee/block_counter as averate_fee,
       to_timestamp(date)::date as date from solana_daily_stat_block;
```

Daily created account
```postgresql
select count(new_account) as new_account, to_timestamp(floor(block_time/86400) * 86400)::date as date
from solana_inst_create_accounts group by  date;
```

Daily transaction number
```postgresql
select solana_daily_stat_block.total_tx, solana_daily_stat_block.success_tx, total_reward/block_counter as average_reward, average_block_time,
       total_fee/block_counter as averate_fee,
       to_timestamp(date)::date as date from solana_daily_stat_block;
```

Daily Transfered lamports
```postgresql
select sum(lamports) as lamports, to_timestamp(floor(block_time/86400) * 86400)::date as date
from solana_inst_transfers group by date;
```
