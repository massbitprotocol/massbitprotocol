/**
 *** This file is to help setup the logger based on the RUST_LOG and RUST_LOG_TYPE options
 **/
use chrono::Local;
use env_logger::{Builder};



use log4rs::append::rolling_file::{policy, RollingFileAppender};
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use std::io::Write;

pub fn log_to_file(file_name: &String, log_level: &String) {
    let one_mb = 1000000;
    let trigger = policy::compound::trigger::size::SizeTrigger::new(one_mb * 100); // unit here is Byte

    // 25-8-2021: Hughie
    // Lazily concat string so we get log with the name of component
    // We can't use format! because we need the {}, maybe try with string escape later
    let owned_string_one: String = "log/".to_owned();
    let owned_string_two: String = (owned_string_one + file_name).to_owned();
    let name_with_gz_extension: String = owned_string_two.clone() + ".{}.gz";
    let name_with_log_extension: String = owned_string_two + ".log";

    let roller = policy::compound::roll::fixed_window::FixedWindowRoller::builder()
        .build(name_with_gz_extension.as_str(), 10000)
        .unwrap(); // We could reach up to 1TB with 100 MB per file * 10.000 files
    let policy = policy::compound::CompoundPolicy::new(Box::new(trigger), Box::new(roller));
    let file = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S.%3f %Z)} {l} [{t} - {T}] {m}{n}",
        )))
        .build(name_with_log_extension, Box::new(policy))
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(file)))
        .build(
            Root::builder()
                .appender("logfile")
                .build(log_level.parse().unwrap()),
        )
        .unwrap();
    log4rs::init_config(config).unwrap();
}

pub fn log_to_console(log_level: &String) {
    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - [{}] {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S"), // Reformat to human-readable timestamp
                record.level(),
                record.module_path_static().unwrap_or_default(),
                record.args(),
            )
        })
        .filter(None, log_level.parse().unwrap())
        .init();
}

pub fn message(output_type: &String, level: &String) -> String {
    format!(
        "Logger will now output to {} with the level: {}",
        output_type, level
    )
}
