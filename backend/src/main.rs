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
//!
//! The web server owns an SQLite database that only its own Linux user can access,
//! holding e.g. the RCON password. The game server has no access to that database;
//! instead, it fetches the RCON password from the web server over a server bound to
//! the local loopback interface only, never on a non-loopback network interface.

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
    pub struct WebServerConfig {
        pub rcon_host: &'static str,
        pub rcon_port: u16,
    }

    impl Default for WebServerConfig {
        fn default() -> Self {
            Self {
                rcon_host: "127.0.0.1",
                rcon_port: launcher::RCON_PORT,
            }
        }
    }

    /// This function first reads the RCON password from [`STATE_DB_PATH`], an
    /// SQLite database that only this web server process ever opens, generating
    /// and storing a random password there if none is stored yet. It then
    /// runs three independent loops until the process stops: RCON connection,
    /// Unix domain socket reader, and RCON password server (to communicate the
    /// password to the separate game server launching process).
    ///
    /// - *RCON connection:* connect to the running game server's RCON WebSocket
    ///   API and repeatedly query the in-game world time (`env.time`), as a
    ///   healthiness heartbeat signal. Re-connect if the connection is lost.
    ///
    /// - *Unix domain socket reader:* Consume data from the Unix domain socket
    ///   that the instrumented game server writes to.
    ///
    /// - *RCON password server:* serve the RCON password to the game server,
    ///   which has no access to the database itself. This is served on the
    ///   local loopback interface only, never on a non-loopback network
    ///   interface.
    ///
    /// Each loop restarts itself on failure, independently of the other.
    /// This function, and hence the managing web server, can be stopped and
    /// restarted without having to restart the game server, and vice versa. The
    /// game and its managing webserver are run as separate `systemd` units.
    pub fn launch_web_server(config: &WebServerConfig) -> std::process::ExitCode {
        let rcon_password: std::sync::Arc<str> = match ensure_rcon_password() {
            Ok(rcon_password) => std::sync::Arc::from(rcon_password),
            Err(err) => {
                std::eprintln!("failed to read or generate RCON password: {err}");
                return std::process::ExitCode::from(21);
            }
        };

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

        async_runtime.block_on(async move {
            let rcon_task = tokio::task::spawn(rcon_heartbeat_loop(
                rcon_host,
                rcon_port,
                std::sync::Arc::clone(&rcon_password),
            ));
            let socket_task = tokio::task::spawn(unix_socket_consumer_loop());
            let password_server_task =
                tokio::task::spawn(password_server_loop(std::sync::Arc::clone(&rcon_password)));

            let _ = rcon_task.await;
            let _ = socket_task.await;
            let _ = password_server_task.await;
        });

        std::process::ExitCode::SUCCESS
    }

    async fn rcon_heartbeat_loop(host: String, port: u16, rcon_password: std::sync::Arc<str>) {
        loop {
            let url: String = std::format!("ws://{host}:{port}/{rcon_password}");

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

                let command = crate::rcon::Message::new("env.time");

                match command
                    .send(&mut websocket_stream, std::time::Duration::from_secs(10))
                    .await
                {
                    Ok(_response) => {}
                    Err(_err) => break,
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
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

    /*
     * TODO:
     *
     *   Use axum instead of a loop, since we're gonna depend on axum in the
     *   program anyway. I.e., create an axum web server for this local loopback
     *   purpose only. There will be a separate axum server instance in this
     *   same process for the public web app serving. Also add additional
     *   security: this local loopback server should check that the connected
     *   client's address is also on the local loopback interface.
     */
    async fn password_server_loop(rcon_password: std::sync::Arc<str>) {
        loop {
            let listener = match tokio::net::TcpListener::bind((
                "127.0.0.1",
                launcher::RCON_PASSWORD_LOCAL_PORT,
            ))
            .await
            {
                Ok(listener) => listener,
                Err(_err) => {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            loop {
                let (mut tcp_stream, _addr) = match listener.accept().await {
                    Ok(accepted) => accepted,
                    Err(_err) => break,
                };

                let rcon_password = std::sync::Arc::clone(&rcon_password);

                tokio::task::spawn(async move {
                    let _ = tokio::io::AsyncWriteExt::write_all(
                        &mut tcp_stream,
                        rcon_password.as_bytes(),
                    )
                    .await;
                    let _ = tokio::io::AsyncWriteExt::shutdown(&mut tcp_stream).await;
                });
            }
        }
    }

    const STATE_DB_PATH: &str = "/srv/rustctl/web/state.sqlite3";

    fn ensure_rcon_password() -> Result<String, String> {
        let connection =
            rusqlite::Connection::open(STATE_DB_PATH).map_err(|err| err.to_string())?;

        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|err| err.to_string())?;

        connection
            .execute("PRAGMA journal_mode = WAL", [])
            .map_err(|err| err.to_string())?;

        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS rcon_password (password TEXT NOT NULL)",
                [],
            )
            .map_err(|err| err.to_string())?;

        let existing_password: Option<String> = rusqlite::OptionalExtension::optional(
            connection.query_row("SELECT password FROM rcon_password LIMIT 1", [], |row| {
                row.get(0)
            }),
        )
        .map_err(|err| err.to_string())?;

        if let Some(existing_password) = existing_password {
            return Ok(existing_password);
        }

        let generated_password = generate_random_password();

        connection
            .execute(
                "INSERT INTO rcon_password (password) VALUES (?1)",
                [&generated_password],
            )
            .map_err(|err| err.to_string())?;

        Ok(generated_password)
    }

    fn generate_random_password() -> String {
        fn random_u64() -> u64 {
            let random_state = std::collections::hash_map::RandomState::new();
            let mut hasher = std::hash::BuildHasher::build_hasher(&random_state);
            std::hash::Hasher::write_u8(&mut hasher, 0);
            std::hash::Hasher::finish(&hasher)
        }

        std::format!("{:016x}{:016x}", random_u64(), random_u64())
    }
}

mod rcon {
    pub(crate) type WebSocketStream = tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >;

    #[derive(serde::Serialize, serde::Deserialize)]
    #[allow(non_snake_case)]
    pub struct Message {
        Identifier: i32,
        Message: String,
    }

    impl Message {
        pub fn new(command: &str) -> Self {
            Self {
                Identifier: generate_random_id(),
                Message: command.to_string(),
            }
        }

        /// Send this message over WebSocket, and wait up to given timeout for
        /// the response.
        ///
        /// RCON response is transmitted in the same WebSocket connection as
        /// the request, interleaved with unrelated broadcast messages (such
        /// as console log lines), so responses are matched to their request
        /// by `Identifier` rather than simply taking whichever message arrives
        /// next.
        pub async fn send(
            &self,
            websocket: &mut WebSocketStream,
            timeout: std::time::Duration,
        ) -> Result<Message, String> {
            let request_json: String =
                serde_json::to_string(self).map_err(|err| err.to_string())?;

            futures_util::SinkExt::send(
                websocket,
                tokio_tungstenite::tungstenite::Message::text(request_json),
            )
            .await
            .map_err(|err| err.to_string())?;

            tokio::time::timeout(timeout, Self::receive_matching(websocket, self.Identifier))
                .await
                .map_err(|_elapsed| "RCON response timed out".to_string())?
        }

        async fn receive_matching(
            websocket: &mut WebSocketStream,
            identifier: i32,
        ) -> Result<Message, String> {
            loop {
                let websocket_message = futures_util::StreamExt::next(websocket)
                    .await
                    .ok_or_else(|| "RCON WebSocket connection closed".to_string())?
                    .map_err(|err| err.to_string())?;

                let response_text = match websocket_message {
                    tokio_tungstenite::tungstenite::Message::Text(text) => text,
                    _ => continue,
                };

                let response: Message = match serde_json::from_str(&response_text) {
                    Ok(response) => response,
                    Err(_err) => continue,
                };

                if response.Identifier == identifier {
                    return Ok(response);
                }
            }
        }
    }

    /// The RCON identifier must presumably fit in a signed 32-bit integer.
    ///
    /// Evidence: error seen in `RustDedicated` buildid `19600410` (2025-08-27):
    ///
    /// ```text
    /// JsonReaderException: JSON integer 3921165172 is too large or small for an Int32. Path 'Identifier', line 1, position 24.
    /// ```
    ///
    /// There's no spec, as far as I'm aware, but let's also assume it has to be
    /// a non-negative integer.
    fn generate_random_id() -> i32 {
        /*
         * Generating a random value using only standard library.
         */
        let random_state = std::collections::hash_map::RandomState::new();
        let mut hasher = std::hash::BuildHasher::build_hasher(&random_state);
        std::hash::Hasher::write_u8(&mut hasher, 0);

        // possibly negative
        let value: i32 = std::hash::Hasher::finish(&hasher) as i32;

        // make non-negative by clearing the high bit
        let value_bounded: i32 = value & 0x_7fff_ffff_i32;

        value_bounded
    }
}
