use chrono::Local;
use env_logger::{Builder, Env};
use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use std::io::Write;

pub fn log_to_file(file_name: &String) {
    let date = Local::now().format("%Y-%m-%dT%H:%M:%S");
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::default()))
        .build(format!("log/{}-{}.log", file_name, date)) // set the file name based on the current date
        .unwrap();
    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))
        .unwrap();
    log4rs::init_config(config).unwrap();
}

pub fn default_logging() {
    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - [{}] {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S"), // Reformat to human-readable timestamp
                record.level(),
                record.module_path_static().unwrap(),
                record.args(),
            )
        })
        .filter(None, LevelFilter::Info)
        .init();
}
