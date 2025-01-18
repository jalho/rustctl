mod args;
mod error;
mod ext_ops;
mod misc;
mod proc;

static EXIT_OK: i32 = 0;

/// Some critical dependency of the program is missing.
static EXIT_ERR_DEPENDENCY_MISSING: i32 = 42;

/// SteamCMD game server installer process or the game server process itself
/// (RustDedicated) is already running.
static EXIT_ERR_PARALLEL_EXECUTION: i32 = 43;

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

                /*
                 * TODO: Check whether RustDedicated is already installed
                 *
                 *       --> Yes: Check for updates:
                 *           ```
                 *           $ steamcmd app_info_update 1
                 *           $ steamcmd app_info_print 258550
                 *           ```
                 *           Then extract the build number and compare it
                 *           against the value in the app manifest: `steamapps/
                 *           appmanifest_258550.acf` under the server install tree.
                 *
                 *       --> No: Install
                 */

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
