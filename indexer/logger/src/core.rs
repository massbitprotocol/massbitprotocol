use std::io::Write;
use chrono::Local;
use env_logger::Builder;
use log::LevelFilter;
// This would add timestamp to every logger
pub fn init_logger() {
    Builder::new()
        .format(|buf, record| {
            writeln!(buf,
                     "{} [{}] - {}",
                     Local::now().format("%Y-%m-%dT%H:%M:%S"),
                     record.level(),
                     record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();
}