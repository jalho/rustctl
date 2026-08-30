//! This program serves two functionalities, each intended to be run as a separate
//! `systemd` unit on a modern Ubuntu system: running a game server, and running
//! a web server for a web app for managing the game server. Each should be
//! independently restartable. Both are defined in the same code base, distinguished
//! at startup by subcommand given via command line interface.

fn main() -> std::process::ExitCode {
    let cli: cli::Cli = <cli::Cli as clap::Parser>::parse();

    match cli.command {
        cli::Command::Game => {
            let cfg = launcher::GameServerConfig::default();
            launcher::launch_game_server(&cfg)
        }

        cli::Command::Web => {
            let cfg = web::WebServerConfig::default();
            web::launch_web_server(&cfg)
        }
    }
}

mod cli {
    #[derive(clap::Parser)]
    pub struct Cli {
        #[command(subcommand)]
        pub command: Command,
    }

    #[derive(clap::Subcommand)]
    pub enum Command {
        /// Launch game server.
        Game,

        /// Launch web server, for a web app for managing the game server.
        Web,
    }
}

mod web {
    pub struct WebServerConfig {}

    impl Default for WebServerConfig {
        fn default() -> Self {
            Self {}
        }
    }

    pub fn launch_web_server(_config: &WebServerConfig) -> std::process::ExitCode {
        std::process::ExitCode::from(45)
    }
}
