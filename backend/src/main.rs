//! This program serves two functionalities, each intended to be run as a separate
//! `systemd` unit on a modern Ubuntu system: running a game server, and running
//! a web server for a web app for managing the game server. Each should be
//! independently restartable. Both are defined in the same code base, distinguished
//! at startup by subcommand given via command line interface.
//!
//! The game server gives information of itself to the managing web server via a
//! Unix domain socket. The game (_Rust_) is instrumented with a modding framework
//! (_Carbon_), for which we define a plugin that writes information about the game
//! server's state into the Unix domain socket that the managing web server then
//! reads. The modding framework takes care of detecting changes in the game's state
//! and passing the information to the plugin.

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
