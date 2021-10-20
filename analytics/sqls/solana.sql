select solana_daily_stat_block.total_tx, solana_daily_stat_block.success_tx, total_reward/block_counter as average_reward, average_block_time,
       total_fee/block_counter as averate_fee,
       to_timestamp(date)::date as date from solana_daily_stat_block;

select count(new_account), to_timestamp(floor(block_time/86400) * 86400)::date as date
from solana_inst_create_accounts group by date;

select sum(lamports) as lamports, to_timestamp(floor(block_time/86400) * 86400)::date as date
from solana_inst_transfers group by date;