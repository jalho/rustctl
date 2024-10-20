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
            match misc::install_steamcmd(&config) {
                Err(err) => {
                    log::error!("{}", err);
                    return Err(err);
                }
                _ => {}
            };

            match misc::install_update_game_server(&config) {
                Err(err) => {
                    log::error!("{}", err);
                    return Err(err);
                }
                _ => {}
            }

            match misc::install_carbon(&config) {
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
                &config,
                config.game_server_argv.iter().map(|s| s.as_str()).collect(),
            ) {
                Ok(n) => n,
                Err(err) => {
                    log::error!("{}", err);
                    return Err(err);
                }
            };

            let (th_stdout_rx, th_stderr_rx) =
                misc::handle_game_fs_events(rx_stdout, rx_stderr, &config);

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
