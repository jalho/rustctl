fn main() -> std::process::ExitCode {
    let _handle = logging::init_logging(log::LevelFilter::Trace);
    log::info!(
        "{name} {version}",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    );

    let _cancellation_token = tokio_util::sync::CancellationToken::new();
    let (_cancel_tx, _cancel_rxx) = tokio::sync::mpsc::channel::<()>(1);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    let _coroutines_done = runtime.block_on(async {
        let gssm = game_server::GameServerStateMachine::init().await;
        let web_server = web::WebServer::init(gssm);
        /*
         * TODO: Add an axum WebSocket server that accepts WebSocket clients.
         *       For each accepted WebSocket client, spawn a tokio Task in which
         *       two things are done repeateadly in a loop:
         *
         *         1. Receive messages from the client: These messages are
         *            considered "commands".
         *
         *         2. Send the current game server state to the client
         *            regularly. Use the tokio "interval".
         */

        // TODO: Use game_server::GameServerStateMachine::handle_command(gssm.clone()).await;
        // TODO: Use game_server::GameServerStateMachine::read_state(gssm.clone()).await;
    });

    let code: std::process::ExitCode = std::process::ExitCode::SUCCESS;
    return code;
}

mod web {
    #[derive(Clone)]
    struct WebServerState {
        game_server_state_machine:
            std::sync::Arc<tokio::sync::RwLock<crate::game_server::GameServerStateMachine>>,
    }

    pub struct WebServer {
        router: axum::Router,
    }

    impl WebServer {
        pub fn init(
            game_server_state_machine: std::sync::Arc<
                tokio::sync::RwLock<crate::game_server::GameServerStateMachine>,
            >,
        ) -> Self {
            let state = WebServerState {
                game_server_state_machine,
            };
            let router = axum::Router::new()
                .route(
                    rustctl_common::web_app::WEBSOCKET_CONNECT_URL_PATH,
                    axum::routing::get(handlers::ws_upgrade),
                )
                .with_state(state);
            Self { router }
        }
    }

    mod handlers {
        pub async fn ws_upgrade(
            ws: axum::extract::WebSocketUpgrade,
            _connect_info: axum::extract::ConnectInfo<std::os::unix::net::SocketAddr>,
            _state: axum::extract::State<super::WebServerState>,
        ) -> impl axum::response::IntoResponse {
            ws.on_upgrade(async move |websocket| {
                let websocket: axum::extract::ws::WebSocket = websocket;
                log::debug!("Do stuff with WebSocket: {websocket:?}");
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
        transitioned_into_at: chrono::DateTime<chrono::Utc>,
        value: T,
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

    #[allow(dead_code)]
    pub struct GameServerStateMachine {
        state: GameServerState,
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
                GameServerState::NotRunning(_) => {
                    let next = Self::new_state(());
                    log::debug!(
                        "Transition: NotRunning -> Wiping @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::Wiping(next)
                }
                GameServerState::Wiping(_) => {
                    let next = Self::new_state(());
                    log::debug!(
                        "Transition: Wiping -> Updating @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::Updating(next)
                }
                GameServerState::Updating(_) => {
                    let next = Self::new_state(());
                    log::debug!(
                        "Transition: Updating -> Launching @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::Launching(next)
                }
                GameServerState::Launching(_) => {
                    let next = Self::new_state(());
                    log::debug!(
                        "Transition: Launching -> RunningHealthy @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::RunningHealthy(next)
                }
                GameServerState::RunningHealthy(_) => {
                    let next = Self::new_state(());
                    log::debug!(
                        "Transition: RunningHealthy -> Stopping @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::Stopping(next)
                }
                GameServerState::Stopping(_) => {
                    let next = Self::new_state(());
                    log::debug!(
                        "Transition: Stopping -> NotRunning @ {}",
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
    }
}
