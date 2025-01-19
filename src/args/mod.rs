//! Abstractions related to the inputs of the CLI program.

#[derive(clap::Parser)]
#[command(name = env!("CARGO_PKG_NAME"), about = "Tooling for hosting a Rust (the game) server.")]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(clap::Subcommand)]
pub enum Cmd {
    #[command(about = "Commands for managing the game server process.")]
    Game {
        #[command(subcommand)]
        cmd: Game,
    },
}

#[derive(clap::Subcommand)]
pub enum Game {
    #[command(
        name = "start",
        about = "Install and update the game server, and then configure and start it."
    )]
    Start,
}
