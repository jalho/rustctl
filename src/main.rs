use error::ErrPrecondition;

mod args;
mod error;
mod ext_ops;
mod misc;
mod proc;

static EXIT_OK: i32 = 0;

/// Some critical dependency of the program is missing, insufficient permissions
/// to filesystem etc.
static EXIT_ERR_SYSTEM_PRECONDITION: i32 = 42;

/// Something that is not supposed to be run in parallel is already running:
/// E.g. the game server or its installer.
static EXIT_ERR_PARALLEL_EXECUTION: i32 = 43;

/// Game server installer failed.
static EXIT_ERR_GAME_INSTALLER: i32 = 44;

/// Game server failed.
static EXIT_ERR_GAME_SERVER: i32 = 45;

static GAME_SERVER_STEAM_APP_ID: u32 = 258550;

fn main() {
    _ = crate::misc::init_logger();

    let cli: crate::args::RustCtlCli = clap::Parser::parse();
    match cli.command {
        crate::args::CliCommand::Game { subcommand: action } => match action {
            crate::args::CliSubCommandGame::InstallUpdateConfigureStart { skip_install } => {
                let installation_dir: &std::path::Path = std::path::Path::new("/home/rust/");
                let steamcmd: crate::proc::Dependency = match crate::proc::Dependency::init(
                    "steamcmd",
                    &installation_dir,
                    String::from("game server installer"),
                    crate::proc::DependencyKind::Other,
                ) {
                    Ok(n) => n,
                    Err(err) => {
                        log::error!("Unrecoverable error: {}", err);
                        std::process::exit(EXIT_ERR_SYSTEM_PRECONDITION);
                    }
                };

                if let Some(pid) = crate::proc::is_process_running(&steamcmd.executable) {
                    log::error!("Unrecoverable error: {steamcmd} is already running: PID {pid}");
                    std::process::exit(EXIT_ERR_PARALLEL_EXECUTION);
                }

                let rustdedicated: crate::proc::Dependency = match crate::proc::Dependency::init(
                    // TODO: Construct the magic path from statics
                    "/home/rust/RustDedicated",
                    installation_dir,
                    String::from("game server"),
                    // TODO: Construct the magic id from statics
                    crate::proc::DependencyKind::SteamApp(GAME_SERVER_STEAM_APP_ID),
                ) {
                    Ok(preinstalled) => {
                        let maybe_updated =
                            match crate::ext_ops::update_game(&steamcmd, &preinstalled) {
                                Ok(n) => n,
                                Err(err) => {
                                    log::error!(
                                        "Unrecoverable error: Could not update RustDedicated: {}",
                                        err
                                    );
                                    std::process::exit(EXIT_ERR_GAME_INSTALLER);
                                }
                            };
                        match maybe_updated {
                            Some(updated) => {
                                log::info!("Updated {} to version {}", &updated, &updated.version);
                                updated
                            }
                            None => {
                                log::info!(
                                    "Dependency {} is up to date: Version {}",
                                    &preinstalled,
                                    &preinstalled.version
                                );
                                preinstalled
                            }
                        }
                    }
                    Err(ErrPrecondition::MissingExecutableDependency(_)) => {
                        let installed =
                            match crate::ext_ops::install_game(&steamcmd, &installation_dir) {
                                Ok(n) => n,
                                Err(err) => {
                                    log::error!(
                                        "Unrecoverable error: Could not install RustDedicated: {}",
                                        err
                                    );
                                    std::process::exit(EXIT_ERR_GAME_INSTALLER);
                                }
                            };
                        log::info!("Dependency installed: {installed}");
                        installed
                    }
                    Err(ErrPrecondition::Filesystem(_)) => todo!(),
                };

                if let Some(pid) = crate::proc::is_process_running(&rustdedicated.executable) {
                    log::error!(
                        "Unrecoverable error: {rustdedicated} is already running: PID {pid}"
                    );
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
                    std::process::exit(EXIT_ERR_GAME_SERVER);
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
