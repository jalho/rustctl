//! Abstractions related to the inputs of the CLI program.

#[derive(clap::Parser)]
#[command(name = env!("CARGO_PKG_NAME"), about = "Tooling for hosting a Rust (the game) server.")]
pub struct RustCtlCli {
    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(clap::Subcommand)]
pub enum CliCommand {
    #[command(about = "Commands for managing the game server process.")]
    Game {
        #[command(subcommand)]
        subcommand: CliSubCommandGame,
    },
}

#[derive(clap::Subcommand)]
pub enum CliSubCommandGame {
    #[command(
        name = "start",
        about = "Install and update the game server (RustDedicated) with SteamCMD and then configure and start the game server."
    )]
    InstallUpdateConfigureStart {
        #[arg(
            long,
            help = "Skip installing (or updating) the game server: Useful for fast iteration during development."
        )]
        skip_install: bool,
    },
}
