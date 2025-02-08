//! Abstractions for managing logging.

#[derive(Debug)]
pub enum Error {
    Cfg(log4rs::config::runtime::ConfigErrors),
    Set(log::SetLoggerError),
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Cfg(err) => Some(err),
            Error::Set(err) => Some(err),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "logger initialization failed")
    }
}
impl From<log4rs::config::runtime::ConfigErrors> for Error {
    fn from(value: log4rs::config::runtime::ConfigErrors) -> Self {
        Self::Cfg(value)
    }
}
impl From<log::SetLoggerError> for Error {
    fn from(value: log::SetLoggerError) -> Self {
        Self::Set(value)
    }
}

fn make_logger_config(
    level: log::LevelFilter,
) -> Result<log4rs::Config, log4rs::config::runtime::ConfigErrors> {
    let stdout: log4rs::append::console::ConsoleAppender =
        log4rs::append::console::ConsoleAppender::builder()
            .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
                "[{d(%Y-%m-%dT%H:%M:%S)}] {h([{l}])} [{t}] - {m}{n}",
            )))
            .build();

    let logger_config: log4rs::Config = log4rs::Config::builder()
        .appender(log4rs::config::Appender::builder().build("stdout", Box::new(stdout)))
        .build(
            log4rs::config::Root::builder()
                .appender("stdout")
                .build(level),
        )?;

    return Ok(logger_config);
}

/// Initialize a global logging utility.
pub fn init_logger(level: log::LevelFilter) -> Result<log4rs::Handle, Error> {
    let config: log4rs::Config = make_logger_config(level)?;
    let handle: log4rs::Handle = log4rs::init_config(config)?;
    return Ok(handle);
}
