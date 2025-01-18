mod args;
mod error;
mod ext_ops;
mod misc;
mod proc;

static EXIT_OK: i32 = 0;

/// Some critical dependency of the program is missing.
static EXIT_ERR_DEPENDENCY_MISSING: i32 = 42;

/// SteamCMD process or RustDedicated process or something else that is not
/// supposed to be run in parallel is already running.
static EXIT_ERR_PARALLEL_EXECUTION: i32 = 43;

/// SteamCMD failed.
static EXIT_ERR_STEAMCMD: i32 = 44;

/// RustDedicated failed.
static EXIT_ERR_RUSTDEDICATED: i32 = 45;

fn main() {
    _ = crate::misc::init_logger();

    let cli: crate::args::RustCtlCli = clap::Parser::parse();
    match cli.command {
        crate::args::CliCommand::Game { subcommand: action } => match action {
            crate::args::CliSubCommandGame::InstallUpdateConfigureStart { skip_install } => {
                let steamcmd: crate::proc::Dependency =
                    match crate::proc::Dependency::init("steamcmd") {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!("Unrecoverable error: {}", err);
                            std::process::exit(EXIT_ERR_DEPENDENCY_MISSING);
                        }
                    };

                if let Some(pid) = crate::proc::is_process_running(&steamcmd.executable) {
                    log::error!("Unrecoverable error: SteamCMD is already running: PID {pid}");
                    std::process::exit(EXIT_ERR_PARALLEL_EXECUTION);
                }

                let rustdedicated: crate::proc::Dependency = match {
                    if crate::ext_ops::is_game_installed() {
                        crate::ext_ops::update_game(&steamcmd)
                    } else {
                        crate::ext_ops::install_game(&steamcmd)
                    }
                } {
                    Ok(n) => n,
                    Err(err) => {
                        log::error!(
                            "Unrecoverable error: Could not install or update RustDedicated: {}",
                            err
                        );
                        std::process::exit(EXIT_ERR_STEAMCMD);
                    }
                };
                if let Some(pid) = crate::proc::is_process_running(&rustdedicated.executable) {
                    log::error!("Unrecoverable error: RustDedicated is already running: PID {pid}");
                    std::process::exit(EXIT_ERR_PARALLEL_EXECUTION);
                }

                /*
                 * TODO: Install or update Carbon modding framework
                 */

                /*
                 * TODO: Install own Carbon plugins
                 */

                let (tx_game_stdout, rx_game_stdout) = std::sync::mpsc::channel::<String>();
                if let Err(err) = crate::ext_ops::run_game(&rustdedicated, tx_game_stdout) {
                    log::error!("Unrecoverable error: Could not run RustDedicated: {}", err);
                    std::process::exit(EXIT_ERR_RUSTDEDICATED);
                }

                /*
                 * TODO: Configure the running game server with Carbon to not be categorized as modded
                 */

                /*
                 * TODO: Signal readiness once done: Write to some Unix domain socket, make INFO log?
                 */
            }
        },
    }

    std::process::exit(EXIT_OK);
}
