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
                let steamcmd_cli: crate::proc::Dependency =
                    match crate::proc::Dependency::init("steamcmd") {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!("Unrecoverable error: {}", err);
                            std::process::exit(EXIT_ERR_DEPENDENCY_MISSING);
                        }
                    };

                if let Err(_) = crate::ext_ops::assure_not_running() {
                    log::error!(
                        "Unrecoverable error: SteamCMD or RustDedicated is already running"
                    );
                    std::process::exit(EXIT_ERR_PARALLEL_EXECUTION);
                }

                if let Err(err) = {
                    if crate::ext_ops::is_game_installed() {
                        crate::ext_ops::update_game(&steamcmd_cli)
                    } else {
                        crate::ext_ops::install_game(&steamcmd_cli)
                    }
                } {
                    log::error!(
                        "Unrecoverable error: Could not install or update RustDedicated: {}",
                        err
                    );
                    std::process::exit(EXIT_ERR_STEAMCMD);
                }

                /*
                 * TODO: Install or update Carbon modding framework
                 */

                /*
                 * TODO: Install own Carbon plugins
                 */

                /*
                 * TODO: Start the game server
                 */

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
