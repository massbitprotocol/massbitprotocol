use chrono::Local;
use env_logger::Builder;
use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

/**
*** The objective of this file is to setup logger writing to file
**/
use std::io::Write;
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
        .filter(None, LevelFilter::Info);
    // .init();

    let date = chrono::Utc::now();
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::default()))
        .build(format!("log/{}.log", date)) // set the file name based on the current date
        .unwrap();
    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))
        .unwrap();
    log4rs::init_config(config).unwrap();
}
