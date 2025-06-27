#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum GameState {
    Initial,
    InstallingOrUpdating,
    ConfiguringGame,
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

struct GameProcess {
    child: Option<tokio::process::Child>,
}

impl GameProcess {
    fn new() -> Self {
        Self { child: None }
    }

    async fn spawn_game(&mut self) {
        if self.child.is_some() {
            todo!("game process already running");
        }

        // TODO: Spawn actual `RustDedicated` executable
        let child = tokio::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .unwrap();

        self.child = Some(child);
    }

    async fn is_running(&mut self) -> bool {
        match &mut self.child {
            Some(child) => match child.try_wait() {
                Ok(Some(_)) => {
                    self.child = None;
                    false
                }
                Ok(None) => true,
                Err(_) => {
                    self.child = None;
                    false
                }
            },
            None => false,
        }
    }

    async fn terminate(&mut self) {
        if let Some(mut child) = self.child.take() {
            child.kill().await.unwrap();
            child.wait().await.unwrap();
        }
    }
}

struct GameManager {
    state: GameState,
    process: GameProcess,
    broadcaster: tokio::sync::broadcast::Sender<StateUpdate>,
}

impl GameManager {
    fn new(broadcaster: tokio::sync::broadcast::Sender<StateUpdate>) -> Self {
        Self {
            state: GameState::Initial,
            process: GameProcess::new(),
            broadcaster,
        }
    }

    async fn transition_to(&mut self, new_state: GameState) {
        if self.state != new_state {
            println!("State transition: {:?} -> {:?}", self.state, new_state);
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
                    self.process.terminate().await;
                    self.transition_to(GameState::GameTerminatedManually).await;
                }
            }
        }
    }

    // TODO: Remove demo sleep: Implement actual transitions
    async fn start_game_launch_sequence(&mut self) {
        self.transition_to(GameState::InstallingOrUpdating).await;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        self.transition_to(GameState::ConfiguringGame).await;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        self.transition_to(GameState::LaunchingGame).await;
        self.process.spawn_game().await;

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        self.transition_to(GameState::GameRunningHealthy).await;
    }

    async fn check_process_health(&mut self) {
        if matches!(self.state, GameState::GameRunningHealthy) {
            if !self.process.is_running().await {
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

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // for sending state updates to clients
    let (broadcast_tx, _) = tokio::sync::broadcast::channel(100);

    let game_server_mgr = std::sync::Arc::new(tokio::sync::RwLock::new(GameManager::new(
        broadcast_tx.clone(),
    )));

    // initial game server startup sequence
    let mgr_autostart = game_server_mgr.clone();
    tokio::spawn(async move {
        let mut mgr = mgr_autostart.write().await;
        mgr.start_game_launch_sequence().await;
    });

    let mgr_health = game_server_mgr.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            let mut mgr = mgr_health.write().await;
            mgr.check_process_health().await;
            mgr.handle_automatic_restart().await;
        }
    });

    let router = axum::Router::new()
        .route("/ws", axum::routing::get(websocket_handler))
        .with_state(game_server_mgr);

    let tcp_listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("WebSocket endpoint: ws://127.0.0.1:3000/ws");

    axum::serve(tcp_listener, router).await.unwrap();
}
