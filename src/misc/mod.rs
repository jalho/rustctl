//! Dumpster for miscellaneous stuff yet to be better categorized.

fn make_logger_config() -> log4rs::Config {
    let stdout: log4rs::append::console::ConsoleAppender =
        log4rs::append::console::ConsoleAppender::builder()
            .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
                "[{d(%Y-%m-%dT%H:%M:%S)}] {h([{l}])} [{t}] - {m}{n}",
            )))
            .build();

    let logger_config: log4rs::Config = match log4rs::Config::builder()
        .appender(log4rs::config::Appender::builder().build("stdout", Box::new(stdout)))
        .build(
            log4rs::config::Root::builder()
                .appender("stdout")
                .build(log::LevelFilter::Trace),
        ) {
        Ok(n) => n,
        Err(_) => {
            unreachable!("logger configuration does not depend on any input so it should be either always valid or never valid");
        }
    };

    return logger_config;
}

/// Initialize a global logging utility.
pub fn init_logger() -> log4rs::Handle {
    let config: log4rs::Config = make_logger_config();
    let logger: log4rs::Handle = match log4rs::init_config(config) {
        Ok(n) => n,
        Err(_) => {
            unreachable!("logger initialization should always succeed because we only do it once");
        }
    };
    return logger;
}
