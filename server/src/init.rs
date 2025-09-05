#[derive(clap::Parser, Debug)]
#[command(version)]
pub struct CliArgs {
    /// Skip updates.
    #[arg(short = 's', long, default_value_t = false)]
    pub skip: bool,

    #[arg(short, long, default_value_t = log::LevelFilter::Debug)]
    pub log_level: log::LevelFilter,

    #[arg(short = 'i', long, default_value_t = DEFAULT_WEB_SERVER_LISTEN_IP_ADDR)]
    pub web_server_listen_ip_addr: std::net::IpAddr,

    #[arg(short = 'p', long, default_value_t = 8080)]
    pub web_server_listen_port: u16,
}

pub const LOG_TARGET_GAME: &str = "game";

pub fn initialize_logger(
    level: log::LevelFilter,
    config: &crate::storage::Configuration,
) -> Result<(log4rs::Handle, String), std::process::ExitCode> {
    const APPENDER_NAME_CORE_FILE: &str = "core_file";
    const APPENDER_NAME_GAME_FILE: &str = "game_file";
    const APPENDER_NAME_STDOUT: &str = "stdout";

    let mut log_file_path = std::path::Path::new(&config.fs.root_dir_abs_utf8()).to_path_buf();
    log_file_path.push("rustctl.log");

    // core -> file
    let appender_core_file = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "{h({d(%Y-%m-%d %H:%M:%S)(utc)} UTC [rustctl] [{l}] {m})} [{f}:{L}]\n",
        )))
        .append(true)
        .build(&log_file_path)
        .map_err(|err| {
            eprintln!("Failed to create core file appender: {err}");
            std::process::ExitCode::FAILURE
        })?;

    // game -> file
    let appender_game_file = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "{h({d(%Y-%m-%d %H:%M:%S)(utc)} UTC [{t}] {m})}\n",
        )))
        .append(true)
        .build(&log_file_path)
        .map_err(|err| {
            eprintln!("Failed to create game file appender: {err}");
            std::process::ExitCode::FAILURE
        })?;

    // core -> stdout
    let appender_stdout = log4rs::append::console::ConsoleAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "{h({d(%Y-%m-%d %H:%M:%S)(utc)} UTC [rustctl] [{l}] {m})}\n",
        )))
        .build();

    let appender_cfg_core_file =
        log4rs::config::Appender::builder().build(APPENDER_NAME_CORE_FILE, Box::new(appender_core_file));
    let appender_cfg_game_file =
        log4rs::config::Appender::builder().build(APPENDER_NAME_GAME_FILE, Box::new(appender_game_file));
    let appender_cfg_stdout =
        log4rs::config::Appender::builder().build(APPENDER_NAME_STDOUT, Box::new(appender_stdout));

    let config = match log4rs::Config::builder()
        .appender(appender_cfg_core_file)
        .appender(appender_cfg_game_file)
        .appender(appender_cfg_stdout)
        /*
         * Game target: Only to game file appender.
         */
        .logger(
            log4rs::config::Logger::builder()
                .appender(APPENDER_NAME_GAME_FILE)
                .additive(false)
                .build(crate::init::LOG_TARGET_GAME, level),
        )
        /*
         * Root (core): To core file + stdout.
         */
        .build(
            log4rs::config::Root::builder()
                .appender(APPENDER_NAME_CORE_FILE)
                .appender(APPENDER_NAME_STDOUT)
                .build(level),
        ) {
        Ok(n) => n,
        Err(err) => {
            eprintln!("Building logger config failed: {err}");
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    match log4rs::init_config(config) {
        Ok(handle) => Ok((handle, log_file_path.to_string_lossy().into_owned())),
        Err(err) => {
            eprintln!("Initializing logger failed: {err}");
            Err(std::process::ExitCode::FAILURE)
        }
    }
}

pub fn build_runtime() -> Result<tokio::runtime::Runtime, std::process::ExitCode> {
    let runtime: tokio::runtime::Runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
    {
        Ok(n) => n,
        Err(err) => {
            log::error!("Failed to build async runtime: {err}");
            return Err(std::process::ExitCode::FAILURE);
        }
    };
    Ok(runtime)
}

const DEFAULT_WEB_SERVER_LISTEN_IP_ADDR: std::net::IpAddr = std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1));
