fn main() -> std::process::ExitCode {
    let cli: temp::Cli = <temp::Cli as clap::Parser>::parse();

    match cli.command {
        temp::Command::Game => {
            let cfg = launcher::GameServerConfig::default();
            launcher::launch_game_server(&cfg)
        }
    }
}

mod temp {
    #[derive(clap::Parser)]
    pub struct Cli {
        #[command(subcommand)]
        pub command: Command,
    }

    #[derive(clap::Subcommand)]
    pub enum Command {
        /// Launch game server.
        Game,
    }
}
