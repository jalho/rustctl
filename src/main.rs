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

    let argv: Vec<String> = std::env::args().collect();
    let command: args::Command = match args::Command::get(argv) {
        Ok(n) => n,
        Err(err) => {
            log::error!("{}", err);
            return Err(err);
        }
    };

    match command {
        args::Command::Help => {
            println!("{}", text::HELPTEXT);
            return Ok(());
        }
        args::Command::Version => {
            println!("{}", text::INFOTEXT);
            return Ok(());
        }
        _ => {}
    }

    let config: args::Config = match args::Config::get_from_fs(args::Config::default_fs_path()) {
        Ok(n) => n,
        Err(err) => {
            log::error!("{}", err);
            return Err(err);
        }
    };

    match command {
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

            match misc::install_update_game_server(
                &config.rustctl_root_dir,
                &config.steamcmd_executable_name,
            ) {
                Err(err) => {
                    log::error!("{}", err);
                    return Err(err);
                }
                _ => {}
            }
        }
        args::Command::HealthStart => todo!(),
        args::Command::Help => todo!(),
        args::Command::Version => todo!(),
        args::Command::WebStart => todo!(),
    }

    return Ok(());
}
