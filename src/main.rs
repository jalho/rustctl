mod core;
mod logging;
mod parsing;
mod system;
mod util;

static EXIT_OK: u8 = 0;
static EXIT_ERR_LOGGER: u8 = 42;
static EXIT_ERR_OTHER: u8 = 43;

fn main() -> std::process::ExitCode {
    let cli: crate::parsing::Cli = <crate::parsing::Cli as clap::Parser>::parse();

    let _handle: log4rs::Handle = match crate::logging::init_logger() {
        Ok(n) => n,
        Err(err) => {
            eprintln!("{}", crate::util::aggregate_error_tree(&err, 2));
            return std::process::ExitCode::from(EXIT_ERR_LOGGER);
        }
    };

    match cli.subcommand {
        crate::parsing::Subcommand::GameStart { exclude } => {
            let game: crate::core::Game = match crate::core::Game::start(exclude) {
                Ok(n) => n,
                Err(err) => {
                    /* TODO:
                     * Check if error case works: "Running parallel" (Multiple
                     * processes called "RustDedicated" already running)
                     */
                    log::error!(
                        "Cannot start game: {}",
                        crate::util::aggregate_error_tree(&err, 2)
                    );
                    return std::process::ExitCode::from(EXIT_ERR_OTHER);
                }
            };
            log::info!("Game started: {game}");
        }
    }

    return std::process::ExitCode::from(EXIT_OK);
}
