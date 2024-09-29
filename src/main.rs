mod args;
mod error;
mod http;
mod misc;
mod text;

fn main() -> Result<(), error::FatalError> {
    misc::init_logger();
    log::debug!("Logger initialized");

    let argv: Vec<String> = std::env::args().collect();
    let config: args::Config = args::Config::get_from_fs(args::Config::default_fs_path())?;

    match args::Command::get(argv)? {
        args::Command::Config => todo!(),
        args::Command::GameStart => {
            /* TODO: Only download SteamCMD if necessary */
            let download_size: usize = get_steamcmd(&config.download_url_steamcmd, &config.rustctl_root_dir)?;
            log::debug!("Downloaded SteamCMD: {} bytes from {}", download_size, config.download_url_steamcmd);
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
fn get_steamcmd(
    url: &String,
    download_dir: &std::path::PathBuf,
) -> Result<usize, http::HttpError> {
    let mut response: std::net::TcpStream = http::request(url)?;
    /* TODO: Extract the .tgz */
    /* TODO: Assert expected entry point exists (steamcmd.sh or something) */
    let mut download_dir = download_dir.clone();
    download_dir.push("steamcmd.tgz");
    let streamed_size: usize = http::stream_to_disk(&mut response, &download_dir)?;
    return Ok(streamed_size);
}
