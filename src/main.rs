mod args;
mod error;
mod misc;
mod text;

fn main() -> Result<(), error::FatalError> {
    match misc::init_logger() {
        Err(err) => {
            log::error!("{}", err);
            return Err(err);
        }
        _ => {}
    };
    log::debug!("Logger initialized");

    let argv: Vec<String> = std::env::args().collect();
    let config: args::Config = match args::Config::get_from_fs(args::Config::default_fs_path()) {
        Ok(n) => n,
        Err(err) => {
            log::error!("{}", err);
            return Err(err);
        }
    };

    match args::Command::get(argv)? {
        args::Command::Config => todo!(),
        args::Command::GameStart => {
            match misc::install_steamcmd(
                &config.steamcmd_download_url,
                &config.rustctl_root_dir,
                &config.steamcmd_target_file_name_tgz,
                &config.steamcmd_executable_name,
            ) {
                Err(err) => {
                    log::error!("{}", err);
                    return Err(err);
                }
                _ => {}
            };
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
