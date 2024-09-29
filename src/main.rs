mod args;
mod error;
mod http;
mod text;

fn main() -> Result<(), error::FatalError> {
    let stdout = log4rs::append::console::ConsoleAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "[{d(%Y-%m-%dT%H:%M:%S%.3fZ)}] [{l}] - {m}{n}",
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
    _ = log4rs::init_config(logger_config).unwrap();
    use log::info;
    info!("Logger configured");

    let argv: Vec<String> = std::env::args().collect();
    let config: args::Config = args::Config::get_from_fs(args::Config::default_fs_path())?;

    match args::Command::get(argv)? {
        args::Command::Config => todo!(),
        args::Command::GameStart => {
            /* TODO: Only download SteamCMD if necessary */
            let _ = download_steamcmd(config.download_url_steamcmd, &config.rustctl_root_dir)?;
        }
        args::Command::HealthStart => todo!(),
        args::Command::Help => {
            println!("{}", text::HELPTEXT);
        }
        args::Command::Version => {
            println!("{}", text::INFOTEXT);
        }
        args::Command::WebStart => todo!(),
    };

    return Ok(());
}

/// Download _SteamCMD_ (game server installer).
fn download_steamcmd(
    url: String,
    download_dir: &std::path::PathBuf,
) -> Result<(), http::HttpError> {
    let mut response: std::net::TcpStream = http::request(url)?;
    /* TODO: Extract the .tgz */
    /* TODO: Assert expected entry point exists (steamcmd.sh or something) */
    let mut download_dir = download_dir.clone();
    download_dir.push("steamcmd.tgz");
    http::stream_to_disk(&mut response, &download_dir)?;
    return Ok(());
}
