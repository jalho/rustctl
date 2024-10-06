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
                &config.steamcmd_installations_dir_name,
                &config.game_server_executable_name,
            ) {
                Err(err) => {
                    log::error!("{}", err);
                    return Err(err);
                }
                _ => {}
            }

            let (tx_stdout, rx_stdout) = std::sync::mpsc::channel::<String>();
            let (tx_stderr, rx_stderr) = std::sync::mpsc::channel::<String>();
            let (th_stdout_tx, th_stderr_tx) = match misc::start_game(
                tx_stdout,
                tx_stderr,
                &config.rustctl_root_dir,
                &config.steamcmd_installations_dir_name,
                config.game_server_executable_name,
                config.game_server_argv.iter().map(|s| s.as_str()).collect(),
            ) {
                Ok(n) => n,
                Err(err) => {
                    log::error!("{}", err);
                    return Err(err);
                }
            };

            let mut game_server_cwd: std::path::PathBuf = config.rustctl_root_dir.clone();
            game_server_cwd.push(config.steamcmd_installations_dir_name);
            let (th_stdout_rx, th_stderr_rx) =
                misc::handle_game_fs_events(rx_stdout, rx_stderr, game_server_cwd);

            _ = th_stdout_tx.join();
            _ = th_stderr_tx.join();
            _ = th_stdout_rx.join();
            _ = th_stderr_rx.join();
        }
        args::Command::HealthStart => todo!(),
        args::Command::Help => todo!(),
        args::Command::Version => todo!(),
        args::Command::WebStart => todo!(),
    }

    return Ok(());
}
