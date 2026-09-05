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
//!
//! The game server and its managing web server should be run as separate Linux
//! users. The game server user should only have access to the game server
//! executable and its related files. Likewise the web server should only have
//! access to the web assets and the web server executable. Both users need to
//! have access to the shared Unix domain socket. All of these: the users, and the
//! socket, and any necessary dependencies (such as `steamcmd`, i.e. the game server
//! installer), are defined in a `cloud-init` file in the same repository with the
//! source. No extra privileges should be granted to the system users: for example,
//! the web server running user does not need full root privileges, but only the
//! necessary capability to bind a web server to port 443 for TLS.

fn main() -> std::process::ExitCode {
    let cli: cli::Cli = <cli::Cli as clap::Parser>::parse();

    match cli.command {
        /*
         * TODO:
         *
         *   Add static linked ("vendored") SQLite backed by a single state
         *   file in a hard-coded file system path defined in a constant used by
         *   both the game server and the web server. The game server shall read
         *   RCON password from the DB and use it when launching the game, or
         *   generate a password and then use it if no password exists yet from
         *   a previous run. The web server shall read the password from the DB
         *   and use it when connecting to the RCON.
         *
         *   The web server will later be implemented with `axum` libraries and
         *   it will persist its state, whatever it may be, in the same SQLite
         *   DB. Keep that in mind when adding the SQLite.
         *
         *   An idea to be considered: replace the `&cfg` passed to both game
         *   server and web server entry points with a DB client of some sort,
         *   i.e. source all parameters from the database. Intent is to minimize
         *   the amount of required CLI args: ideally, the program will only
         *   read the state from a single DB state file, and any configuration
         *   is done by altering that state. Given a hard-coded DB state file
         *   path, there shouldn't be any need for any additional CLI args,
         *   other than the "subcommand" distinguishing whether running the game
         *   server or the web server systemd unit.
         *
         *   Describe this idea in short also in the crate top level doc
         *   comment.
         */

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
    pub struct WebServerConfig {
        pub rcon_host: &'static str,
        pub rcon_port: u16,
        pub rcon_password: &'static str,
    }

    impl Default for WebServerConfig {
        fn default() -> Self {
            Self {
                rcon_host: "127.0.0.1",
                rcon_port: launcher::RCON_PORT,
                rcon_password: "",
            }
        }
    }

    /// This function runs two independent loops until the process stops: RCON
    /// connection, and Unix domain socket reader.
    ///
    /// - *RCON connection:* connect to the running game server's RCON WebSocket
    ///   API and repeatedly query the in-game world time (`env.time`), as a
    ///   healthiness heartbeat signal. Re-connect if the connection is lost.
    ///
    /// - *Unix domain socket reader:* Consume data from the Unix domain socket
    ///   that the instrumented game server writes to.
    ///
    /// Each loop restarts itself on failure, independently of the other.
    /// This function, and hence the managing web server, can be stopped and
    /// restarted without having to restart the game server, and vice versa. The
    /// game and its managing webserver are run as separate `systemd` units.
    pub fn launch_web_server(config: &WebServerConfig) -> std::process::ExitCode {
        let async_runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(async_runtime) => async_runtime,
            Err(err) => {
                std::eprintln!("failed to start async runtime: {err}");
                return std::process::ExitCode::from(20);
            }
        };

        let rcon_host: String = config.rcon_host.to_owned();
        let rcon_port: u16 = config.rcon_port;
        let rcon_password: String = config.rcon_password.to_owned();

        async_runtime.block_on(async move {
            let rcon_task =
                tokio::task::spawn(rcon_heartbeat_loop(rcon_host, rcon_port, rcon_password));
            let socket_task = tokio::task::spawn(unix_socket_consumer_loop());

            let _ = rcon_task.await;
            let _ = socket_task.await;
        });

        std::process::ExitCode::SUCCESS
    }

    async fn rcon_heartbeat_loop(host: String, port: u16, password: String) {
        loop {
            let url: String = std::format!("ws://{host}:{port}/{password}");

            let mut websocket_stream = match tokio_tungstenite::connect_async(url).await {
                Ok((websocket_stream, _response)) => websocket_stream,
                Err(_err) => {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            let mut heartbeat_interval = tokio::time::interval(std::time::Duration::from_secs(1));
            heartbeat_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                heartbeat_interval.tick().await;

                let request: String = serde_json::json!({
                    "Identifier": random_rcon_identifier(),
                    "Message": "env.time",
                })
                .to_string();

                let send_result = futures_util::SinkExt::send(
                    &mut websocket_stream,
                    tokio_tungstenite::tungstenite::Message::text(request),
                )
                .await;

                if send_result.is_err() {
                    break;
                }

                let heartbeat_result = tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    futures_util::StreamExt::next(&mut websocket_stream),
                )
                .await;

                match heartbeat_result {
                    Ok(Some(Ok(_message))) => {}
                    _ => break,
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }

    /*
     * TODO:
     *
     *   Check the supported range for RCON message identifiers.
     */
    fn random_rcon_identifier() -> i32 {
        let random_state = std::collections::hash_map::RandomState::new();
        let mut hasher = std::hash::BuildHasher::build_hasher(&random_state);
        std::hash::Hasher::write_u8(&mut hasher, 0);
        (std::hash::Hasher::finish(&hasher) as i32) & 0x7fff_ffff
    }

    async fn unix_socket_consumer_loop() {
        loop {
            /*
             * Removing and re-creating the socket associated file path is fine
             * because the plugin is expected to reconnect on its own before its
             * next write, so it recovers on its own, independently of when the
             * web server happens to come back up.
             */
            let _ = std::fs::remove_file(launcher::UNIX_DOMAIN_SOCKET);

            let listener = match tokio::net::UnixListener::bind(launcher::UNIX_DOMAIN_SOCKET) {
                Ok(listener) => listener,
                Err(_err) => {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            loop {
                let (unix_stream, _addr) = match listener.accept().await {
                    Ok(accepted) => accepted,
                    Err(_err) => break,
                };

                tokio::task::spawn(async move {
                    let mut lines =
                        tokio::io::AsyncBufReadExt::lines(tokio::io::BufReader::new(unix_stream));

                    while let Ok(Some(_line)) = lines.next_line().await {}
                });
            }
        }
    }
}
