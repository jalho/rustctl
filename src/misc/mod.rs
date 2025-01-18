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
            /*
             * The configuration is always valid or never valid because it's
             * fully known at compile time and doesn't depend on any inputs.
             */
            unreachable!();
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
            /*
             * Initialization wit valid config should always succeed unless
             * initialized more than once, which we don't do!
             */
            unreachable!();
        }
    };
    return logger;
}
