fn main() {
    let _handle = logging::init_logging(log::LevelFilter::Debug);
    log::info!(
        "{name} {version}",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    );

    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let cancel_web = cancellation_token.child_token();
    let cancel_health = cancellation_token.child_token();

    // for sending state updates to clients
    let (broadcast_tx, _) = tokio::sync::broadcast::channel(100);

    let game_server_mgr = std::sync::Arc::new(tokio::sync::RwLock::new(GameManager::init(
        broadcast_tx.clone(),
    )));
    let mgr_startup = game_server_mgr.clone();
    let mgr_health = game_server_mgr.clone();

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

        _ = tokio::join!(
            coroutine_signal,
            coroutine_startup,
            coroutine_health,
            coroutine_web,
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
}

struct GameManager {
    state: GameState,
    game_server_executable: GameExecutable,
    broadcaster: tokio::sync::broadcast::Sender<StateUpdate>,
}

impl GameManager {
    fn init(broadcaster: tokio::sync::broadcast::Sender<StateUpdate>) -> Self {
        Self {
            state: GameState::Initial,
            game_server_executable: GameExecutable::init(),
            broadcaster,
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
                    if let Some(mut child) = self.game_server_executable.process.take() {
                        child.kill().await.unwrap();
                        child.wait().await.unwrap();
                    }
                    self.transition_to(GameState::GameTerminatedManually).await;
                }
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
            constants::EXECUTABLE_GAME_SERVER_STEAM_APP_ID,
            "validate",
            "+quit",
        ]);
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        log::debug!("Spawning command: {command:?}");
        let output: std::process::Output =
            command.spawn().unwrap().wait_with_output().await.unwrap();
        log::debug!("Output: {output:?}");
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
        let process = tokio::process::Command::new("sleep")
            .current_dir("/home/rust/")
            .args(vec!["5s"])
            .spawn()
            .unwrap();
        self.game_server_executable.process = Some(process);
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
    pub const GAME_SERVER_ROOT: &'static str = "/home/rust/";
    pub const EXECUTABLE_GAME_SERVER: &'static str = "RustDedicated";
    pub const EXECUTABLE_GAME_SERVER_STEAM_APP_ID: &'static str = "258550";
    pub const EXECUTABLE_GAME_SERVER_INSTALLER: &'static str = "steamcmd";
}
