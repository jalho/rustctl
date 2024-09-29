//! Dumpster for miscellaneous stuff yet to be better categorized.

/// Initialize a global logging utility.
pub fn init_logger() -> Result<log4rs::Handle, crate::args::ArgError> {
    let stdout = log4rs::append::console::ConsoleAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "[{d(%Y-%m-%dT%H:%M:%S%.3f)}] [{l}] - {m}{n}",
        )))
        .build();
    let logger_config: log4rs::Config = match log4rs::Config::builder()
        .appender(log4rs::config::Appender::builder().build("stdout", Box::new(stdout)))
        .build(
            log4rs::config::Root::builder()
                .appender("stdout")
                .build(log::LevelFilter::Debug),
        ) {
        Ok(n) => n,
        Err(err) => {
            return Err(crate::args::ArgError::ConfigInvalid(format!(
                "{:?}",
                err.errors()
            )));
        }
    };
    let logger: log4rs::Handle = match log4rs::init_config(logger_config) {
        Ok(n) => n,
        // SetLoggerError is not really an arg error but whatever
        Err(err) => return Err(crate::args::ArgError::ConfigInvalid(format!("{}", err))),
    };
    return Ok(logger);
}

pub enum InstallError {
    HttpError(crate::http::HttpError),
    ExtractError,
}
impl std::fmt::Debug for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HttpError(arg0) => f.debug_tuple("HttpError").field(arg0).finish(),
            Self::ExtractError => write!(f, "ExtractError"),
        }
    }
}
impl From<crate::http::HttpError> for InstallError {
    fn from(err: crate::http::HttpError) -> Self {
        return Self::HttpError(err);
    }
}

/// Install _SteamCMD_ (game server installer).
pub fn install_steamcmd(
    url: &String,
    download_dir: &std::path::PathBuf,
    target_file_name: &String,
) -> Result<(), InstallError> {
    let mut path = download_dir.clone();
    path.push(target_file_name);

    if !path.is_file() {
        let mut response: std::net::TcpStream = crate::http::request(url)?;
        let streamed_size: usize = crate::http::stream_to_disk(&mut response, &path)?;
        log::debug!("Downloaded SteamCMD: {} bytes from {}", streamed_size, url);
    } else {
        log::debug!(
            "SteamCMD distribution '{}' has been downloaded earlier -- Not downloading again",
            path.to_string_lossy()
        );
    }

    /* TODO: Assert expected entry point exists (steamcmd.sh or something): extract the .tgz if not yet extracted */

    return Ok(());
}
