mod args;
mod error;
mod misc;
mod proc;
mod text;

fn main() -> Result<(), error::FatalError> {
    misc::init_logger()?;

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

    let config: args::Config = match args::Config::new() {
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
            let (game_pgid, th_stdout_tx, th_stderr_tx) =
                match misc::start_game(tx_stdout, tx_stderr, &config) {
                    Ok(n) => n,
                    Err(err) => {
                        log::error!("{}", err);
                        return Err(err);
                    }
                };

            let (tx_game_server_state, rx_game_server_state) =
                std::sync::mpsc::channel::<misc::GameServerState>();
            let (th_stdout_rx, th_stderr_rx) = misc::handle_game_server_fs_events(
                &config,
                rx_stdout,
                rx_stderr,
                tx_game_server_state,
            );

            let rcon_websocket = match misc::get_rcon_websocket(rx_game_server_state, &config) {
                Ok(n) => n,
                Err(err) => {
                    log::error!("{}", err);
                    return Err(err);
                }
            };

            if let Err(err) = misc::configure_carbon(rcon_websocket) {
                /* We want to kill a (grand)child process spawned by child
                process (strace), and the only way to do that AFAIK is by using
                process groups over an unsafe libc API. */
                unsafe { libc::killpg(game_pgid, libc::SIGKILL) };
                log::error!("{}", err);
                return Err(err);
            }

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
