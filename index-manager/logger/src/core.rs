use chrono::Local;
use env_logger::Builder;
use log::LevelFilter;
/**
*** The objective of this file is to parse the env logger to the correct format
**/
use std::io::Write;
// This would add timestamp to every logger
pub fn init_logger() {
    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();
}
