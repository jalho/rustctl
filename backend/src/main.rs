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
        /*
         * TODO(LLM):
         *
         *   (Re)connect to the game server's RCON WebSocket API and repeatedly
         *   query the in-game world time (`env.time`), as a healthiness
         *   heartbeat signal. Re-establish connection whenever its lost.
         *   Note that re-starting the game server with installing updates
         *   and re-generating the map may take up to 30 minutes or something.
         *   Also keep consuming data from the Unix domain socket that the
         *   instrumented game server writes to: do whatever housekeeping
         *   is required in that kind of use case: Idk when the socket
         *   file descriptor needs to be re-created and by which side: the
         *   instrumented game server's plugin, or its launcher, or this
         *   managing web server. Note that each of the components must be
         *   manually restartable and each of them shall remain running if any
         *   of the other components are taken down. For example, the managing
         *   web server can be stopped and restarted without having to restart
         *   the game server, and vice versa. Also document these ideas in doc
         *   comment of `fn launch_web_server` similar to how I've instructed in
         *   the TODO of the game server launcher fn.
         */
        std::process::ExitCode::from(45)
    }
}
