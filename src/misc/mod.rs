//! Dumpster for miscellaneous stuff yet to be better categorized.

/// Initialize a global logging utility.
pub fn init_logger() -> log4rs::Handle {
    let stdout = log4rs::append::console::ConsoleAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "[{d(%Y-%m-%dT%H:%M:%S%.3f)}] [{l}] - {m}{n}",
        )))
        .build();
    let logger_config: log4rs::Config = log4rs::Config::builder()
        .appender(log4rs::config::Appender::builder().build("stdout", Box::new(stdout)))
        .build(
            log4rs::config::Root::builder()
                .appender("stdout")
                .build(log::LevelFilter::Debug),
        )
        .unwrap();
    let logger: log4rs::Handle = log4rs::init_config(logger_config).unwrap();
    return logger;
}
