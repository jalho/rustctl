fn main() -> std::process::ExitCode {
    let _handle = logging::init_logging(log::LevelFilter::Trace);
    log::info!(
        "{name} {version}",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    );

    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let (cancel_tx, cancel_rx) = tokio::sync::mpsc::channel::<()>(1);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    let coroutines_done = runtime.block_on(async {
        let game_server_state_machine = game_server::GameServerStateMachine::init().await;
    });

    let code: std::process::ExitCode = std::process::ExitCode::SUCCESS;
    return code;
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
    struct TrackedState<T> {
        transitioned_into_at: i64,
        value: T,
    }

    #[allow(dead_code)]
    #[derive(Clone, Debug)]
    enum GameServerState {
        NotRunning(TrackedState<()>),
        Wiping(TrackedState<()>),
        Updating(TrackedState<()>),
        Launching(TrackedState<()>),
        RunningHealthy(TrackedState<()>),
        Stopping(TrackedState<()>),
    }

    pub struct GameServerStateMachine {
        state: GameServerState,
    }

    impl GameServerStateMachine {
        pub async fn init() -> Self {
            Self {
                state: GameServerState::NotRunning(Self::new_state(())),
            }
        }

        pub fn current_state(&self) -> GameServerState {
            self.state.clone()
        }

        pub async fn transition(&mut self) {
            self.state = match self.state {
                GameServerState::NotRunning(_) => {
                    let next = Self::new_state(());
                    println!(
                        "Transition: NotRunning -> Wiping @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::Wiping(next)
                }
                GameServerState::Wiping(_) => {
                    let next = Self::new_state(());
                    println!(
                        "Transition: Wiping -> Updating @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::Updating(next)
                }
                GameServerState::Updating(_) => {
                    let next = Self::new_state(());
                    println!(
                        "Transition: Updating -> Launching @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::Launching(next)
                }
                GameServerState::Launching(_) => {
                    let next = Self::new_state(());
                    println!(
                        "Transition: Launching -> RunningHealthy @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::RunningHealthy(next)
                }
                GameServerState::RunningHealthy(_) => {
                    let next = Self::new_state(());
                    println!(
                        "Transition: RunningHealthy -> Stopping @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::Stopping(next)
                }
                GameServerState::Stopping(_) => {
                    let next = Self::new_state(());
                    println!(
                        "Transition: Stopping -> NotRunning @ {}",
                        next.transitioned_into_at
                    );
                    GameServerState::NotRunning(next)
                }
            };
        }

        fn new_state<T>(value: T) -> TrackedState<T> {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            TrackedState {
                transitioned_into_at: timestamp,
                value,
            }
        }
    }

    async fn handle_commands(
        state_machine: std::sync::Arc<tokio::sync::RwLock<GameServerStateMachine>>,
    ) {
        loop {
            let _some_inbound_command = ();
            {
                let mut locked = state_machine.write().await;
                match locked.state {
                    GameServerState::NotRunning(ref _state) => {
                        // TODO: Check if the inbound command is compatible with the current state, and only call transition if compatible!
                        locked.transition().await;
                    }
                    GameServerState::Wiping(ref _state) => {
                        // TODO: Check if the inbound command is compatible with the current state, and only call transition if compatible!
                        locked.transition().await;
                    }
                    GameServerState::Updating(ref _state) => {
                        // TODO: Check if the inbound command is compatible with the current state, and only call transition if compatible!
                        locked.transition().await;
                    }
                    GameServerState::Launching(ref _state) => {
                        // TODO: Check if the inbound command is compatible with the current state, and only call transition if compatible!
                        locked.transition().await;
                    }
                    GameServerState::RunningHealthy(ref _state) => {
                        // TODO: Check if the inbound command is compatible with the current state, and only call transition if compatible!
                        locked.transition().await;
                    }
                    GameServerState::Stopping(ref _state) => {
                        // TODO: Check if the inbound command is compatible with the current state, and only call transition if compatible!
                        locked.transition().await;
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
        }
    }

    async fn read_state(
        state_machine: std::sync::Arc<tokio::sync::RwLock<GameServerStateMachine>>,
    ) {
        loop {
            {
                let locked = state_machine.read().await;
                println!("Current state: {:?}", locked.current_state());
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
}
