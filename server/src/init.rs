#[derive(clap::Parser, Debug)]
#[command(version)]
pub struct CliArgs {
    #[arg(short, long, default_value_t = log::LevelFilter::Debug)]
    pub log_level: log::LevelFilter,
}

pub fn initialize_logger(level: log::LevelFilter) -> log4rs::Handle {
    const LOG_TARGET_GAME: &str = "game";
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

    let config = log4rs::Config::builder()
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
        )
        .unwrap();

    log4rs::init_config(config).unwrap()
}
