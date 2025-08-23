#[derive(clap::Parser, Debug)]
#[command(version)]
pub struct CliArgs {
    #[arg(short, long, default_value_t = log::LevelFilter::Debug)]
    pub log_level: log::LevelFilter,

    #[arg(short = 'i', long, default_value_t = WEB_SERVER_LISTEN_IP_ADDR)]
    pub web_server_listen_ip_addr: std::net::IpAddr,

    #[arg(short = 'p', long, default_value_t = 8080)]
    pub web_server_listen_port: u16,
}

pub const LOG_TARGET_GAME: &str = "game";

pub fn initialize_logger(level: log::LevelFilter) -> Result<log4rs::Handle, std::process::ExitCode> {
    const APPENDER_NAME_CORE: &str = "core";
    const APPENDER_NAME_GAME: &str = "game_server";

    let appender_core: log4rs::append::console::ConsoleAppender = log4rs::append::console::ConsoleAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "{h({d(%Y-%m-%d %H:%M:%S)(utc)} [rustctl] {m})} [{f}:{L}]\n",
        )))
        .build();

    let appender_game: log4rs::append::console::ConsoleAppender = log4rs::append::console::ConsoleAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "{h({d(%Y-%m-%d %H:%M:%S)(utc)} [{t}] {m})}\n",
        )))
        .build();

    let appender_cfg_core: log4rs::config::Appender =
        log4rs::config::Appender::builder().build(APPENDER_NAME_CORE, Box::new(appender_core));

    let appender_cfg_game: log4rs::config::Appender =
        log4rs::config::Appender::builder().build(APPENDER_NAME_GAME, Box::new(appender_game));

    let config = match log4rs::Config::builder()
        .appender(appender_cfg_core)
        .appender(appender_cfg_game)
        .logger(
            log4rs::config::Logger::builder()
                .appender(APPENDER_NAME_GAME)
                .additive(false) // log only for the specific target, i.e. don't propagate duplicate log
                .build(LOG_TARGET_GAME, level),
        )
        .build(
            log4rs::config::Root::builder()
                .appender(APPENDER_NAME_CORE)
                .build(level),
        ) {
        Ok(n) => n,
        Err(err) => {
            eprintln!("Building logger config failed: {err}");
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    match log4rs::init_config(config) {
        Ok(n) => Ok(n),
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

const WEB_SERVER_LISTEN_IP_ADDR: std::net::IpAddr = std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1));
