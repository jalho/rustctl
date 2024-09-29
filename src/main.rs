mod args;
mod error;
mod http;
mod misc;
mod text;

fn main() -> Result<(), error::FatalError> {
    misc::init_logger()?;
    log::debug!("Logger initialized");

    let argv: Vec<String> = std::env::args().collect();
    let config: args::Config = args::Config::get_from_fs(args::Config::default_fs_path())?;

    match args::Command::get(argv)? {
        args::Command::Config => todo!(),
        args::Command::GameStart => {
            /* TODO: Only download SteamCMD if necessary */
            let download_size: usize = misc::install_steamcmd(
                &config.steamcmd_download_url,
                &config.rustctl_root_dir,
                &config.steamcmd_target_file_name_tgz,
            )?;
            log::debug!(
                "Downloaded SteamCMD: {} bytes from {}",
                download_size,
                config.steamcmd_download_url
            );
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
