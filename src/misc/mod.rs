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
        Err(_) => {
            return Err(crate::args::ArgError::ConfigInvalid(format!(
                "TODO: Describe log4rs error"
            )));
        }
    };
    let logger: log4rs::Handle = match log4rs::init_config(logger_config) {
        Ok(n) => n,
        Err(_) => {
            return Err(crate::args::ArgError::ConfigInvalid(format!(
                "TODO: Describe log4rs error"
            )))
        }
    };
    return Ok(logger);
}

/// Install _SteamCMD_ (game server installer).
pub fn install_steamcmd(
    url: &String,
    download_dir: &std::path::PathBuf,
) -> Result<usize, crate::http::HttpError> {
    let mut response: std::net::TcpStream = crate::http::request(url)?;
    /* TODO: Extract the .tgz */
    /* TODO: Assert expected entry point exists (steamcmd.sh or something) */
    let mut download_dir = download_dir.clone();
    download_dir.push("steamcmd.tgz");
    let streamed_size: usize = crate::http::stream_to_disk(&mut response, &download_dir)?;
    return Ok(streamed_size);
}
