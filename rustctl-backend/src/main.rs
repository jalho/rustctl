fn main() -> std::process::ExitCode {
    let _handle = logging::init_logging(log::LevelFilter::Debug);
    log::info!(
        "{name} {version}",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    );

    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let (_cancel_tx, cancel_rx) = tokio::sync::mpsc::channel::<()>(1);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        let gssm = game_server::GameServerStateMachine::init().await;

        let web_service = web::WebServer::listen(
            cancellation_token.child_token(),
            "127.0.0.1:8081".parse().unwrap(),
            gssm.clone(),
        );

        let _coroutines_done = tokio::join!(
            tokio::spawn(lifecycle::wait_signal(cancellation_token, cancel_rx)),
            tokio::spawn(game_server::GameServerStateMachine::start(gssm.clone())),
            tokio::spawn(web_service),
        );
    });

    let code: std::process::ExitCode = std::process::ExitCode::SUCCESS;
    code
}

mod lifecycle {
    pub async fn wait_signal(
        cancel: tokio_util::sync::CancellationToken,
        mut shutdown_rx: tokio::sync::mpsc::Receiver<()>,
    ) {
        let mut sigint =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = sigint.recv() => log::info!("SIGINT"),
            _ = sigterm.recv() => log::info!("SIGTERM"),
            _ = shutdown_rx.recv() => log::info!("Shutdown requested by coroutine"),
        }
        cancel.cancel();
    }
}

mod web {
    #[allow(dead_code)]
    #[derive(Clone)]
    struct WebServerState {
        game_server_state_machine:
            std::sync::Arc<tokio::sync::RwLock<crate::game_server::GameServerStateMachine>>,
    }

    pub struct WebServer;

    impl WebServer {
        pub async fn listen(
            cancellation_token: tokio_util::sync::CancellationToken,
            listen_addr: std::net::SocketAddr,
            game_server_state_machine: std::sync::Arc<
                tokio::sync::RwLock<crate::game_server::GameServerStateMachine>,
            >,
        ) -> Result<(), std::io::Error> {
            let state = WebServerState {
                game_server_state_machine,
            };
            let router = axum::Router::new()
                .route(
                    rustctl_common::web_app::WEBSOCKET_CONNECT_URL_PATH,
                    axum::routing::get(handlers::ws_upgrade),
                )
                .with_state(state);
            let server: axum_server::Server = axum_server::bind(listen_addr);
            let service = router.into_make_service_with_connect_info::<std::net::SocketAddr>();
            let done: Option<Result<(), std::io::Error>> = cancellation_token
                .run_until_cancelled(server.serve(service))
                .await;
            match done {
                Some(done) => done,
                None => Ok(()),
            }
        }
    }

    #[allow(dead_code)]
    struct WebSocketClient {
        addr: std::net::SocketAddr,
        tx: futures::stream::SplitSink<axum::extract::ws::WebSocket, axum::extract::ws::Message>,
        rx: futures::stream::SplitStream<axum::extract::ws::WebSocket>,
        gssm: std::sync::Arc<tokio::sync::RwLock<crate::game_server::GameServerStateMachine>>,
    }

    impl WebSocketClient {
        pub fn new(
            addr: std::net::SocketAddr,
            tx: futures::stream::SplitSink<
                axum::extract::ws::WebSocket,
                axum::extract::ws::Message,
            >,
            rx: futures::stream::SplitStream<axum::extract::ws::WebSocket>,
            gssm: std::sync::Arc<tokio::sync::RwLock<crate::game_server::GameServerStateMachine>>,
        ) -> Self {
            Self { addr, tx, rx, gssm }
        }

        pub async fn send_and_receive_messages(self) {
            let coroutine_tx = {
                let mut tx = self.tx;
                let gssm = self.gssm.clone();

                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
                    loop {
                        interval.tick().await;

                        let game_server_state: crate::game_server::GameServerState =
                            crate::game_server::GameServerStateMachine::read_state(gssm.clone())
                                .await;
                        let captured_at: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

                        let sendable_snapshot: rustctl_common::snapshot::Snapshot =
                            Self::make_sendable_snapshot(captured_at, game_server_state);
                        let serialized: String =
                            serde_json::to_string_pretty(&sendable_snapshot).unwrap();

                        match futures::SinkExt::send(&mut tx, serialized.into()).await {
                            Ok(_) => {}
                            Err(_err) => todo!(),
                        }
                    }
                })
            };

            let coroutine_rx = {
                let mut rx = self.rx;
                let gssm = self.gssm.clone();

                tokio::spawn(async move {
                    'recv_messages: while let Some(msg_result) =
                        futures::StreamExt::next(&mut rx).await
                    {
                        match msg_result {
                            Ok(msg) => match msg {
                                axum::extract::ws::Message::Text(text) => {
                                    let command: rustctl_common::command::Command =
                                        serde_json::from_str(&text).unwrap();

                                    let backboard: crate::game_server::GameServerStateBackboard = {
                                        let locked = gssm.read().await;
                                        match locked.state.accepts_command(&command) {
                                            Some(backboard) => backboard,
                                            None => continue 'recv_messages,
                                        }
                                    };

                                    loop {
                                        let mut locked = gssm.write().await;
                                        locked.transition().await;
                                        let current: crate::game_server::GameServerStateBackboard =
                                            (&locked.state).into();
                                        if backboard == current {
                                            break;
                                        }
                                    }
                                }
                                _ => todo!(),
                            },
                            Err(_err) => todo!(),
                        }
                    }
                })
            };

            _ = tokio::join!(coroutine_tx, coroutine_rx)
        }

        fn make_sendable_snapshot(
            captured_at: chrono::DateTime<chrono::Utc>,
            game_server_state: crate::game_server::GameServerState,
        ) -> rustctl_common::snapshot::Snapshot {
            rustctl_common::snapshot::Snapshot {
                captured_at,
                game_server_state: game_server_state.into(),
            }
        }
    }

    impl From<crate::game_server::GameServerState>
        for rustctl_common::snapshot::GameServerStateExposed
    {
        fn from(value: crate::game_server::GameServerState) -> Self {
            match value {
                crate::game_server::GameServerState::NotRunning(s) => {
                    rustctl_common::snapshot::GameServerStateExposed::NotRunning(
                        rustctl_common::snapshot::TrackedState {
                            transitioned_into_at: s.transitioned_into_at,
                            value: s.value,
                        },
                    )
                }
                crate::game_server::GameServerState::Wiping(s) => {
                    rustctl_common::snapshot::GameServerStateExposed::Wiping(
                        rustctl_common::snapshot::TrackedState {
                            transitioned_into_at: s.transitioned_into_at,
                            value: s.value,
                        },
                    )
                }
                crate::game_server::GameServerState::Updating(s) => {
                    rustctl_common::snapshot::GameServerStateExposed::Updating(
                        rustctl_common::snapshot::TrackedState {
                            transitioned_into_at: s.transitioned_into_at,
                            value: s.value,
                        },
                    )
                }
                crate::game_server::GameServerState::Launching(s) => {
                    rustctl_common::snapshot::GameServerStateExposed::Launching(
                        rustctl_common::snapshot::TrackedState {
                            transitioned_into_at: s.transitioned_into_at,
                            value: s.value,
                        },
                    )
                }
                crate::game_server::GameServerState::RunningHealthy(s) => {
                    rustctl_common::snapshot::GameServerStateExposed::RunningHealthy(
                        rustctl_common::snapshot::TrackedState {
                            transitioned_into_at: s.transitioned_into_at,
                            value: s.value,
                        },
                    )
                }
                crate::game_server::GameServerState::Stopping(s) => {
                    rustctl_common::snapshot::GameServerStateExposed::Stopping(
                        rustctl_common::snapshot::TrackedState {
                            transitioned_into_at: s.transitioned_into_at,
                            value: s.value,
                        },
                    )
                }
            }
        }
    }

    mod handlers {
        pub async fn ws_upgrade(
            ws: axum::extract::WebSocketUpgrade,
            connect_info: axum::extract::ConnectInfo<std::net::SocketAddr>,
            state: axum::extract::State<super::WebServerState>,
        ) -> impl axum::response::IntoResponse {
            ws.on_upgrade(async move |websocket| {
                let websocket: axum::extract::ws::WebSocket = websocket;
                let (tx, rx) = futures::StreamExt::split(websocket);
                let client = crate::web::WebSocketClient::new(
                    connect_info.0,
                    tx,
                    rx,
                    state.0.game_server_state_machine.clone(),
                );
                client.send_and_receive_messages().await;
            })
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

mod game_server {
    #[allow(dead_code)]
    #[derive(Clone, Debug)]
    pub struct TrackedState<T> {
        pub transitioned_into_at: chrono::DateTime<chrono::Utc>,
        pub value: T,
    }

    #[allow(dead_code)]
    #[derive(Clone, Debug)]
    pub enum GameServerState {
        NotRunning(TrackedState<()>),
        Wiping(TrackedState<()>),
        Updating(TrackedState<()>),
        Launching(TrackedState<()>),
        RunningHealthy(TrackedState<()>),
        Stopping(TrackedState<()>),
    }

    impl GameServerState {
        /// Returns `None` if the given command is not applicable as a
        /// transition from the current state. Otherwise if the command is
        /// applicable as a transition or as a series of transitions, then the
        /// state until which transitions should be taken is returned.
        pub fn accepts_command(
            &self,
            command: &rustctl_common::command::Command,
        ) -> Option<GameServerStateBackboard> {
            match self {
                GameServerState::Wiping(_)
                | GameServerState::Updating(_)
                | GameServerState::Launching(_)
                | GameServerState::Stopping(_) => None,
                GameServerState::RunningHealthy(_) => {
                    if command == &rustctl_common::command::Command::TransitionFromRunningHealthy {
                        Some(GameServerStateBackboard::NotRunning)
                    } else {
                        None
                    }
                }
                GameServerState::NotRunning(_) => {
                    if command == &rustctl_common::command::Command::TransitionFromNotRunning {
                        Some(GameServerStateBackboard::RunningHealthy)
                    } else {
                        None
                    }
                }
            }
        }
    }

    #[derive(PartialEq, Eq)]
    pub enum GameServerStateBackboard {
        NotRunning,
        Wiping,
        Updating,
        Launching,
        RunningHealthy,
        Stopping,
    }

    impl From<&GameServerState> for GameServerStateBackboard {
        fn from(value: &GameServerState) -> Self {
            match value {
                GameServerState::NotRunning(_) => GameServerStateBackboard::NotRunning,
                GameServerState::Wiping(_) => GameServerStateBackboard::Wiping,
                GameServerState::Updating(_) => GameServerStateBackboard::Updating,
                GameServerState::Launching(_) => GameServerStateBackboard::Launching,
                GameServerState::RunningHealthy(_) => GameServerStateBackboard::RunningHealthy,
                GameServerState::Stopping(_) => GameServerStateBackboard::Stopping,
            }
        }
    }

    #[allow(dead_code)]
    pub struct GameServerStateMachine {
        pub state: GameServerState,
    }

    #[allow(dead_code)]
    impl GameServerStateMachine {
        pub async fn init() -> std::sync::Arc<tokio::sync::RwLock<Self>> {
            std::sync::Arc::new(tokio::sync::RwLock::new(Self {
                state: GameServerState::NotRunning(Self::new_state(())),
            }))
        }

        pub async fn transition(&mut self) {
            self.state = match self.state {
                GameServerState::NotRunning(ref _state) => {
                    let next = Self::new_state(());
                    log::debug!(
                        "Transitioned: NotRunning -> Wiping @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::Wiping(next)
                }

                GameServerState::Wiping(ref _state) => {
                    /*
                     * TODO: Remove old game state: Blueprints, map etc...
                     */
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                    let next = Self::new_state(());
                    log::debug!(
                        "Transitioned: Wiping -> Updating @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::Updating(next)
                }

                GameServerState::Updating(ref _state) => {
                    /*
                     * TODO: Install or update the game server using SteamCMD...
                     */
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                    let next = Self::new_state(());
                    log::debug!(
                        "Transitioned: Updating -> Launching @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::Launching(next)
                }

                GameServerState::Launching(ref _state) => {
                    /*
                     * TODO: Launch the updated game server...
                     */
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                    let next = Self::new_state(());
                    log::debug!(
                        "Transitioned: Launching -> RunningHealthy @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::RunningHealthy(next)
                }

                GameServerState::RunningHealthy(ref _state) => {
                    /*
                     * TODO: Stop the game server...
                     */
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                    let next = Self::new_state(());
                    log::debug!(
                        "Transitioned: RunningHealthy -> Stopping @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::Stopping(next)
                }

                GameServerState::Stopping(ref _state) => {
                    let next = Self::new_state(());
                    log::debug!(
                        "Transitioned: Stopping -> NotRunning @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::NotRunning(next)
                }
            };
        }

        fn new_state<T>(value: T) -> TrackedState<T> {
            TrackedState {
                transitioned_into_at: chrono::Utc::now(),
                value,
            }
        }

        pub async fn handle_command(state_machine: std::sync::Arc<tokio::sync::RwLock<Self>>) {
            let mut locked = state_machine.write().await;
            locked.transition().await;
        }

        pub async fn read_state(
            state_machine: std::sync::Arc<tokio::sync::RwLock<Self>>,
        ) -> GameServerState {
            let locked = state_machine.read().await;
            locked.state.clone()
        }

        /// Start the initial startup sequence of the _game server state
        /// machine_: Install the game server or any available updates and then
        /// launch the game.
        pub async fn start(state_machine: std::sync::Arc<tokio::sync::RwLock<Self>>) {
            loop {
                let mut locked = state_machine.write().await;
                if let GameServerStateBackboard::RunningHealthy = (&locked.state).into() {
                    break;
                } else {
                    locked.transition().await;
                }
            }
        }
    }
}
