const SECONDS_IN_DAY : u64 = 24 * 60 * 60;
pub fn timestamp_round_to_date(timestamp : u64) -> u64{
    timestamp / SECONDS_IN_DAY * SECONDS_IN_DAY
}