fn main() {
    let args: misc::Args = <misc::Args as clap::Parser>::parse();

    let _handle = logging::init_logging(args.log_level);
    log::info!(
        "{name} {version}",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    );

    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let cancel_web = cancellation_token.child_token();
    let cancel_health = cancellation_token.child_token();
    let cancel_cleanup = cancellation_token.child_token();

    // for sending state updates to clients
    let (broadcast_tx, _) = tokio::sync::broadcast::channel(100);

    let game_server_mgr = std::sync::Arc::new(tokio::sync::RwLock::new(GameManager::init(
        broadcast_tx.clone(),
    )));
    let mgr_startup = game_server_mgr.clone();
    let mgr_health = game_server_mgr.clone();
    let mgr_cleanup = game_server_mgr.clone();

    let router = axum::Router::new()
        .route("/ws", axum::routing::get(websocket_handler))
        .with_state(game_server_mgr);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        // program lifecycle
        let coroutine_signal = tokio::spawn(async move {
            let mut sigint =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
            tokio::select! {
                _ = sigint.recv() => log::info!("SIGINT"),
                _ = sigterm.recv() => log::info!("SIGTERM"),
            }
            cancellation_token.cancel();
        });

        // initial game server startup sequence
        let coroutine_startup = tokio::spawn(async move {
            let mut mgr = mgr_startup.write().await;
            mgr.start_game_launch_sequence().await;
        });

        // healthcheck: restart on crash
        let coroutine_health = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            'health_check: loop {
                interval.tick().await;
                if cancel_health.is_cancelled() {
                    break 'health_check;
                }
                let mut mgr = mgr_health.write().await;
                mgr.check_process_health().await;
                mgr.handle_automatic_restart().await;
            }
        });

        // server for WebSocket clients
        let coroutine_web = tokio::spawn(async move {
            let tcp_listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
                .await
                .unwrap();
            cancel_web
                .run_until_cancelled(async move {
                    axum::serve(tcp_listener, router).await.unwrap();
                })
                .await;
        });

        // cleanup
        let coroutine_cleanup = tokio::spawn(async move {
            cancel_cleanup.cancelled().await;

            {
                log::info!("Shutting down game server...");
                let mut mgr = mgr_cleanup.write().await;
                mgr.shutdown_game_server().await;
            }

            log::info!("Cleanup completed");
        });

        _ = tokio::join!(
            coroutine_signal,
            coroutine_startup,
            coroutine_health,
            coroutine_web,
            coroutine_cleanup,
        );
    });
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum GameState {
    Initial,
    InstallingGame,
    InstallingMods,
    LaunchingGame,
    GameRunningHealthy,
    GameTerminatedUnexpectedly,
    GameTerminatedManually,
    GameClosingGracefully,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum ClientCommand {
    InitiateGameLaunchSequence,
    CloseGameGracefully,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StateUpdate {
    state: GameState,
    timestamp: u64,
}

struct GameExecutable {
    process: Option<tokio::process::Child>,
}

impl GameExecutable {
    fn init() -> Self {
        Self { process: None }
    }

    async fn is_running(&mut self) -> bool {
        match &mut self.process {
            Some(child) => match child.try_wait() {
                Ok(Some(_)) => {
                    self.process = None;
                    false
                }
                Ok(None) => true,
                Err(_) => {
                    self.process = None;
                    false
                }
            },
            None => false,
        }
    }

    async fn shutdown(&mut self) {
        if let Some(mut child) = self.process.take() {
            log::debug!("Terminating game server process...");

            if let Err(err) = child.start_kill() {
                log::warn!("Failed to send SIGTERM to game server: {err}");
            } else {
                log::debug!("Sent SIGTERM to game server, waiting for graceful shutdown...");

                match tokio::time::timeout(std::time::Duration::from_secs(10), child.wait()).await {
                    Ok(Ok(exit_status)) => {
                        log::info!("Game server exited gracefully with status: {}", exit_status);
                        return;
                    }
                    Ok(Err(err)) => {
                        log::warn!("Error waiting for game server to exit: {err}");
                    }
                    Err(_) => {
                        log::warn!(
                            "Game server did not exit within 10 seconds, forcing termination..."
                        );
                    }
                }
            }

            if let Err(err) = child.kill().await {
                log::warn!("Failed to force kill game server: {err}");
            } else {
                log::debug!("Force killed game server, waiting for process to be reaped...");
                match child.wait().await {
                    Ok(exit_status) => {
                        log::info!("Game server process reaped with status: {}", exit_status);
                    }
                    Err(err) => {
                        log::error!("Error waiting for game server process to be reaped: {err}");
                    }
                }
            }
        }
    }
}

struct GameConfiguration {
    rcon_port: u16,
    rcon_password: String,
}

struct RconClient {
    tx: futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        tokio_tungstenite::tungstenite::Message,
    >,
    rx: futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
}

impl RconClient {
    pub async fn connect(
        rcon_port: u16,
        rcon_password: &str,
        timeout: &std::time::Duration,
    ) -> Self {
        let ws_url = format!("ws://127.0.0.1:{}/{}", rcon_port, rcon_password);
        log::debug!(
            "Connecting to RCON WebSocket at ws://127.0.0.1:{}/[password]",
            rcon_port
        );

        let connect =
            tokio::time::timeout(*timeout, tokio_tungstenite::connect_async(&ws_url)).await;

        let (ws_stream, _) = match connect {
            Ok(Ok((stream, response))) => {
                log::debug!("Connected to RCON WebSocket");
                (stream, response)
            }
            Ok(Err(err)) => todo!("failed to connect to RCON WebSocket: {err}"),
            Err(err) => todo!("timeout connecting to RCON WebSocket: {err}"),
        };

        let (tx, rx): (
            futures_util::stream::SplitSink<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
                tokio_tungstenite::tungstenite::Message,
            >,
            futures_util::stream::SplitStream<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
            >,
        ) = futures_util::StreamExt::split(ws_stream);
        Self { tx, rx }
    }

    async fn send_command(
        &mut self,
        rcon_command: &str,
        timeout: &std::time::Duration,
    ) -> Result<rcon::RconResponse, ()> {
        let command_identifier: u16 = rand::Rng::random(&mut rand::rng());

        let command_message: String = serde_json::json!({
            "Message": rcon_command,
            "Identifier": command_identifier
        })
        .to_string();

        log::debug!("Sending RCON command: {}", command_message);
        if let Err(err) = futures_util::SinkExt::send(
            &mut self.tx,
            tokio_tungstenite::tungstenite::Message::Text(command_message.into()),
        )
        .await
        {
            todo!("failed to send command: {err}");
        }

        let start_time = std::time::Instant::now();
        'recv_response: loop {
            let elapsed = start_time.elapsed();
            if elapsed >= *timeout {
                todo!(
                    "timeout waiting for command response with identifier {}",
                    command_identifier
                );
            }

            let remaining_timeout = *timeout - elapsed;
            let message_result = tokio::time::timeout(
                remaining_timeout,
                futures_util::StreamExt::next(&mut self.rx),
            )
            .await;

            match message_result {
                Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Text(response)))) => {
                    log::debug!("Received RCON message: {}", response);

                    if let Ok(parsed) = serde_json::from_str::<rcon::RconResponse>(&response) {
                        if parsed.identifier == command_identifier {
                            log::debug!(
                                "Found matching identifier {}, command completed",
                                parsed.identifier
                            );
                            return Ok(parsed);
                        } else {
                            log::debug!(
                                "Received message with identifier {}, expecting {}, continuing to wait",
                                parsed.identifier,
                                command_identifier
                            );
                            continue 'recv_response;
                        }
                    } else {
                        log::debug!("Failed to parse message as RconResponse, continuing to wait");
                        continue 'recv_response;
                    }
                }
                Ok(Some(Ok(other_message))) => {
                    log::debug!(
                        "Received non-text RCON message: {:?}, continuing to wait",
                        other_message
                    );
                    continue 'recv_response;
                }
                Ok(Some(Err(err))) => {
                    todo!("error receiving RCON message: {err}");
                }
                Ok(None) => {
                    todo!("RCON connection closed while waiting for response");
                }
                Err(_) => {
                    continue 'recv_response;
                }
            }
        }
    }
}

struct GameManager {
    game_configuration: GameConfiguration,
    state: GameState,
    game_server_executable: GameExecutable,
    broadcaster: tokio::sync::broadcast::Sender<StateUpdate>,
    rcon_client: Option<RconClient>,
}

impl GameManager {
    fn init(broadcaster: tokio::sync::broadcast::Sender<StateUpdate>) -> Self {
        Self {
            state: GameState::Initial,
            game_server_executable: GameExecutable::init(),
            broadcaster,
            game_configuration: GameConfiguration {
                rcon_port: 28016,
                rcon_password: uuid::Uuid::new_v4().to_string(),
            },
            rcon_client: None,
        }
    }

    async fn transition_to(&mut self, new_state: GameState) {
        log::debug!("State transition: {:?} -> {:?}", self.state, new_state);
        self.state = new_state.clone();

        let update = StateUpdate {
            state: new_state,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let _ = self.broadcaster.send(update);
    }

    async fn handle_command(&mut self, command: ClientCommand) {
        match command {
            ClientCommand::InitiateGameLaunchSequence => {
                if matches!(
                    self.state,
                    GameState::GameTerminatedManually | GameState::GameTerminatedUnexpectedly
                ) {
                    self.start_game_launch_sequence().await;
                }
            }

            ClientCommand::CloseGameGracefully => {
                if matches!(self.state, GameState::GameRunningHealthy) {
                    self.transition_to(GameState::GameClosingGracefully).await;
                    self.game_server_executable.shutdown().await;
                    self.transition_to(GameState::GameTerminatedManually).await;
                }
            }
        }
    }

    async fn shutdown_game_server(&mut self) {
        match self.state {
            GameState::GameRunningHealthy => {
                self.transition_to(GameState::GameClosingGracefully).await;
                self.game_server_executable.shutdown().await;
                self.transition_to(GameState::GameTerminatedManually).await;
            }
            GameState::LaunchingGame => {
                self.game_server_executable.shutdown().await;
                self.transition_to(GameState::GameTerminatedManually).await;
            }
            _ => {
                self.game_server_executable.shutdown().await;
            }
        }
    }

    async fn start_game_launch_sequence(&mut self) {
        self.transition_to(GameState::InstallingGame).await;
        self.install_game_server().await;

        self.transition_to(GameState::InstallingMods).await;
        self.install_carbon_mod_framework().await;
        self.install_carbon_mod_plugin().await;

        self.transition_to(GameState::LaunchingGame).await;
        self.launch_game().await;
        self.connect_rcon().await;
        self.render_map_image_file().await;

        self.transition_to(GameState::GameRunningHealthy).await;
    }

    /// Install (or update) the game server (executable named `RustDedicated`)
    /// using _SteamCMD_ (executable named `steamcmd`).
    ///
    /// Tested with the following `apt` distribution of SteamCMD on Debian 12:
    /// - Package: steamcmd:i386
    /// - Version: 0~20180105-5
    /// - Section: non-free/games
    /// - Maintainer: Debian Games Team
    async fn install_game_server(&self) {
        let mut app_manifest: std::path::PathBuf =
            std::path::Path::new(constants::GAME_SERVER_ROOT).to_path_buf();
        app_manifest.push(constants::GAME_SERVER_STEAM_APP_MANIFEST);

        let steam_app_build_id_before: Option<u32>;
        if let Ok(contents) = tokio::fs::read_to_string(&app_manifest).await {
            match misc::extract_buildid_from_buf(&contents) {
                Some(buildid) => {
                    steam_app_build_id_before = Some(buildid);
                }
                None => todo!(
                    "could not extract Steam app buildid from contents of {}: {}",
                    app_manifest.to_string_lossy(),
                    contents
                ),
            }
        } else {
            steam_app_build_id_before = None;
        }

        let mut command = tokio::process::Command::new(constants::EXECUTABLE_GAME_SERVER_INSTALLER);
        command.current_dir(constants::GAME_SERVER_ROOT);
        command.args(vec![
            "+login",
            "anonymous",
            /*
             * TODO: "force_install_dir" doesn't really "force" anything:
             *       Instead, SteamCMD seems to just create a new directory tree
             *       in "~/.local/share/Steam/" if it cannot access the given
             *       "force_install_dir"... Therefore, we must add some checks
             *       to actually know where the installation ends up at...
             */
            "+force_install_dir",
            constants::GAME_SERVER_ROOT,
            "+app_update",
            constants::GAME_SERVER_STEAM_APP_ID,
            "validate",
            "+quit",
        ]);
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        match steam_app_build_id_before {
            Some(buildid) => log::info!(
                "Currently installed game server buildid: {buildid} -- Checking for updates..."
            ),
            None => log::info!("Installing game server..."),
        }
        let output: std::process::Output =
            command.spawn().unwrap().wait_with_output().await.unwrap();
        if !output.status.success() {
            todo!("{output:?}");
        }

        let steam_app_build_id_after: u32;
        let contents: String = tokio::fs::read_to_string(&app_manifest)
            .await
            .expect("app manifest should exist after installation");
        match misc::extract_buildid_from_buf(&contents) {
            Some(buildid) => {
                steam_app_build_id_after = buildid;
            }
            None => todo!(
                "could not extract Steam app buildid from contents of {}: {}",
                app_manifest.to_string_lossy(),
                contents
            ),
        }

        match steam_app_build_id_before {
            Some(before) => {
                if before != steam_app_build_id_after {
                    log::info!(
                        "Game server updated from buildid {before} to {steam_app_build_id_after}"
                    );
                } else {
                    log::debug!(
                        "Game server not updated: Already at latest version: buildid {before}"
                    )
                }
            }
            None => log::info!("Game server installed: buildid {steam_app_build_id_after}"),
        }
    }

    /// Install (or update) _CarbonModding Framework_ from some HTTP repository.
    async fn install_carbon_mod_framework(&self) {
        // TODO: Implement!
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    /// Install a plugin for _CarbonModding Framework_ from some HTTP
    /// repository.
    async fn install_carbon_mod_plugin(&self) {
        // TODO: Implement!
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    /// Launch the installed game server (executable named `RustDedicated`).
    async fn launch_game(&mut self) {
        let mut executable: std::path::PathBuf =
            std::path::Path::new(constants::GAME_SERVER_ROOT).to_path_buf();
        executable.push(constants::EXECUTABLE_GAME_SERVER);

        let mut command = tokio::process::Command::new(executable);
        command.current_dir(constants::GAME_SERVER_ROOT);

        /*
         * As of 2025-06-30, the game server expects a `steamclient.so` that
         * seems to get installed in the same directory as the game server
         * executable itself, but must be specified with an env var for some
         * reason anyway!
         */
        command.env("LD_LIBRARY_PATH", constants::GAME_SERVER_ROOT);

        command.args(vec![
            "-batchmode",
            "+server.port",
            "28015",
            "+server.level",
            "Procedural Map",
            "+server.worldsize",
            "1000",
            "+rcon.port",
            &self.game_configuration.rcon_port.to_string(),
            "+rcon.password",
            &self.game_configuration.rcon_password.to_string(),
            "+rcon.web",
            "1",
        ]);
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<()>();

        log::info!("Launching game server...");
        let mut process: tokio::process::Child = command.spawn().unwrap();

        let stdout = process.stdout.take().unwrap();
        tokio::spawn(async move {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let mut ready_tx = Some(ready_tx);
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                log::debug!("[{}:STDOUT] {line}", constants::EXECUTABLE_GAME_SERVER);
                /*
                 * As of 2025-06-30, the latest version of the game server emits
                 * to STDOUT e.g. the following lines when it seems ready:
                 * ```
                 * SteamServer Initialized
                 * Server startup complete
                 * SteamServer Connected
                 * ```
                 */
                if line.contains("SteamServer Connected") {
                    if let Some(tx) = ready_tx.take() {
                        let _ = tx.send(());
                    }
                }
            }
        });

        let stderr = process.stderr.take().unwrap();
        tokio::spawn(async move {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                log::debug!("[{}:STDERR] {line}", constants::EXECUTABLE_GAME_SERVER);
            }
        });

        self.game_server_executable.process = Some(process);

        let _ = ready_rx.await;
        log::info!("Game server is ready");
    }

    async fn connect_rcon(&mut self) {
        let rcon_client: RconClient = RconClient::connect(
            self.game_configuration.rcon_port,
            &self.game_configuration.rcon_password,
            &std::time::Duration::from_secs(2),
        )
        .await;
        self.rcon_client = Some(rcon_client);
    }

    // TODO: Use actual Mutex instead of Option::take & re-assign?
    async fn render_map_image_file(&mut self) {
        let rcon_response: rcon::RconResponse;
        {
            let mut rcon_client: RconClient = self.rcon_client.take().unwrap();
            rcon_response = rcon_client
                .send_command("rendermap", &std::time::Duration::from_secs(10))
                .await
                .unwrap();
            self.rcon_client = Some(rcon_client);
        }

        let (path_from, _metadata): (std::path::PathBuf, std::fs::Metadata) =
            rcon::verify_rendered_map(&rcon_response).await;

        let mut path_to = std::path::Path::new(constants::GAME_SERVER_ROOT).to_path_buf();
        path_to.push(constants::GAME_WORLD_MAP_RENDERED_IMAGE);
        tokio::fs::rename(&path_from, &path_to).await.unwrap();

        log::info!(
            "Game world map rendered as {} and then renamed as {}",
            path_from.to_string_lossy(),
            path_to.to_string_lossy(),
        );
    }

    async fn check_process_health(&mut self) {
        if matches!(self.state, GameState::GameRunningHealthy) {
            if !self.game_server_executable.is_running().await {
                self.transition_to(GameState::GameTerminatedUnexpectedly)
                    .await;
            }
        }
    }

    async fn handle_automatic_restart(&mut self) {
        if matches!(self.state, GameState::GameTerminatedUnexpectedly) {
            self.start_game_launch_sequence().await;
        }
    }
}

type SharedManager = std::sync::Arc<tokio::sync::RwLock<GameManager>>;

async fn websocket_handler(
    ws: axum::extract::WebSocketUpgrade,
    axum::extract::State(manager): axum::extract::State<SharedManager>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| handle_socket(socket, manager))
}

async fn handle_socket(socket: axum::extract::ws::WebSocket, manager: SharedManager) {
    let (mut sender, mut receiver) = futures_util::StreamExt::split(socket);

    // subscribe to state updates
    let mut rx = {
        let mgr = manager.read().await;
        mgr.broadcaster.subscribe()
    };

    // send current state immediately
    {
        let mgr = manager.read().await;
        let current_state = StateUpdate {
            state: mgr.state.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        if let Ok(msg) = serde_json::to_string(&current_state) {
            let _ = futures_util::SinkExt::send(
                &mut sender,
                axum::extract::ws::Message::Text(msg.into()),
            )
            .await;
        }
    }

    // send state updates
    let manager_clone = manager.clone();
    tokio::spawn(async move {
        while let Ok(update) = rx.recv().await {
            if let Ok(msg) = serde_json::to_string(&update) {
                if futures_util::SinkExt::send(
                    &mut sender,
                    axum::extract::ws::Message::Text(msg.into()),
                )
                .await
                .is_err()
                {
                    break;
                }
            }
        }
    });

    // handle incoming commands
    while let Some(msg) = futures_util::StreamExt::next(&mut receiver).await {
        if let Ok(msg) = msg {
            if let axum::extract::ws::Message::Text(text) = msg {
                if let Ok(command) = serde_json::from_str::<ClientCommand>(&text) {
                    let mut mgr = manager_clone.write().await;
                    mgr.handle_command(command).await;
                }
            }
        }
    }
}

mod logging {
    pub fn init_logging(level: log::LevelFilter) -> log4rs::Handle {
        let stdout = log4rs::append::console::ConsoleAppender::builder()
            .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
                "{h({d(%Y-%m-%dT%H:%M:%SZ)(utc)} {l} - {m})} [{f}:{L}] [{T}]\n",
            )))
            .build();

        let name = "stdout";

        let config = log4rs::Config::builder()
            .appender(log4rs::config::Appender::builder().build(name, Box::new(stdout)))
            .build(log4rs::config::Root::builder().appender(name).build(level))
            .unwrap();

        log4rs::init_config(config).unwrap()
    }
}

mod constants {
    /// Absolute path to the directory where the game server should be installed..
    pub const GAME_SERVER_ROOT: &'static str = "/home/rust/";

    /// File name only, not full path.
    pub const EXECUTABLE_GAME_SERVER: &'static str = "RustDedicated";

    /// File name only, not full path.
    pub const EXECUTABLE_GAME_SERVER_INSTALLER: &'static str = "steamcmd";

    pub const GAME_SERVER_STEAM_APP_ID: &'static str = "258550";
    /// Path relative to the location of the game server executable.
    pub const GAME_SERVER_STEAM_APP_MANIFEST: &'static str = "steamapps/appmanifest_258550.acf";

    /// File name only, not full path.
    pub const GAME_WORLD_MAP_RENDERED_IMAGE: &'static str = "map-current.png";
}

mod misc {
    #[derive(clap::Parser, Debug)]
    #[command(version)]
    pub struct Args {
        #[arg(short, long, default_value_t = log::LevelFilter::Debug)]
        pub log_level: log::LevelFilter,
    }

    pub fn extract_buildid_from_buf(buf: &str) -> Option<u32> {
        let vdf: keyvalues_parser::Vdf = match keyvalues_parser::Vdf::parse(buf) {
            Ok(v) => v,
            Err(_) => return None,
        };
        let root: &keyvalues_parser::Obj = match vdf.value.get_obj() {
            Some(n) => n,
            None => return None,
        };

        let buildid_str: &str = match root.get("buildid") {
            Some(values) => {
                if values.len() != 1 {
                    todo!("expected exactly one buildid value, found {}", values.len());
                }
                match values[0].get_str() {
                    Some(s) => s,
                    None => return None,
                }
            }
            None => return None,
        };

        match buildid_str.parse::<u32>() {
            Ok(n) => Some(n),
            Err(_) => None,
        }
    }
}

mod rcon {
    #[derive(Debug, serde::Deserialize)]
    pub struct RconResponse {
        #[serde(rename = "Message")]
        pub message: String,
        #[serde(rename = "Identifier")]
        pub identifier: u16,
    }

    pub async fn verify_rendered_map(
        rcon_response: &RconResponse,
    ) -> (std::path::PathBuf, std::fs::Metadata) {
        let msg: &String = &rcon_response.message;
        let path: &str = msg.strip_prefix("Saved map render to: ").unwrap();
        let path: std::path::PathBuf = tokio::fs::canonicalize(path).await.unwrap();
        let metadata: std::fs::Metadata = tokio::fs::metadata(&path).await.unwrap();
        return (path, metadata);
    }
}
