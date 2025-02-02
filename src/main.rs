mod core;
mod logging;
mod parsing;
mod system;
mod util;

fn main() -> std::process::ExitCode {
    let cli: crate::parsing::Cli = <crate::parsing::Cli as clap::Parser>::parse();

    let _handle: log4rs::Handle = match crate::logging::init_logger() {
        Ok(n) => n,
        Err(err) => {
            eprintln!("{}", crate::util::aggregate_error_tree(&err, 2));
            return std::process::ExitCode::FAILURE;
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
                    return std::process::ExitCode::FAILURE;
                }
            };
            log::info!("Game started: {game}");
        }
    }

    return std::process::ExitCode::SUCCESS;
}
