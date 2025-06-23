fn main() -> std::process::ExitCode {
    let coordinator: std::sync::Arc<temp::Coordinator> = temp::Coordinator::init();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    let coroutines_done = runtime.block_on(async {
        tokio::join!(
            coordinator.clone().start_sl(),
            coordinator.clone().start_srum(),
            coordinator.clone().start_gssm(),
            coordinator.clone().start_gws(),
            coordinator.clone().start_ws(),
        )
    });

    let code: std::process::ExitCode = temp::CoroutinesTerminated::capture(coroutines_done).into();
    return code;
}

mod temp {
    use std::{process::ExitCode, sync::Arc};
    use tokio::{
        sync::Mutex,
        task::{JoinError, JoinHandle},
    };
    use tokio_util::sync::CancellationToken;

    pub struct CoroutinesTerminated {
        results: Vec<Result<Result<(), NRE>, JoinError>>,
    }

    impl CoroutinesTerminated {
        pub fn capture(
            results: (
                Result<Result<(), NRE>, JoinError>,
                Result<Result<(), NRE>, JoinError>,
                Result<Result<(), NRE>, JoinError>,
                Result<Result<(), NRE>, JoinError>,
                Result<Result<(), NRE>, JoinError>,
            ),
        ) -> Self {
            let (a, b, c, d, e) = results;
            Self {
                results: vec![a, b, c, d, e],
            }
        }
    }

    impl From<CoroutinesTerminated> for ExitCode {
        fn from(value: CoroutinesTerminated) -> Self {
            'results: for result in value.results {
                match result {
                    Ok(Ok(ok)) => {
                        let _coroutine_ok: () = ok;
                        continue 'results;
                    }
                    Ok(Err(err)) => {
                        let _err: NRE = err;
                        return ExitCode::FAILURE;
                    }
                    Err(err) => {
                        let _err: JoinError = err;
                        return ExitCode::FAILURE;
                    }
                }
            }
            return ExitCode::SUCCESS;
        }
    }

    /// A _non-recoverable error_ (NRE).
    #[derive(Debug)]
    pub enum NRE {
        MissingRequiredDependency,
    }

    struct SystemResourcesUsage;

    impl SystemResourcesUsage {
        pub fn init() -> Self {
            todo!()
        }
    }

    struct WebClientsConnected;

    impl WebClientsConnected {
        pub fn init() -> Self {
            todo!()
        }
    }

    enum GameServerStateMachine {}

    impl GameServerStateMachine {
        pub fn init() -> Self {
            todo!()
        }
    }

    struct GameWorldSnapshot;

    impl GameWorldSnapshot {
        pub fn init() -> Self {
            todo!()
        }
    }

    pub struct Coordinator {
        cancellation_token: CancellationToken,
        system_resources_usage: Mutex<SystemResourcesUsage>,
        web_clients_connected: Mutex<WebClientsConnected>,
        game_server_state_machine: Mutex<GameServerStateMachine>,
        game_world_snapshot: Mutex<GameWorldSnapshot>,
    }

    impl Coordinator {
        pub fn init() -> Arc<Self> {
            Arc::new(Self {
                cancellation_token: CancellationToken::new(),
                system_resources_usage: Mutex::new(SystemResourcesUsage::init()),
                web_clients_connected: Mutex::new(WebClientsConnected::init()),
                game_server_state_machine: Mutex::new(GameServerStateMachine::init()),
                game_world_snapshot: Mutex::new(GameWorldSnapshot::init()),
            })
        }

        /// Start _signal listener_ ("sl"): Activate the CancellationToken in
        /// `self` on SIGINT, SIGTERM, or whenever any of the peer coroutines
        /// use the `mpsc::channel` in `self` to signal to terminate.
        pub fn start_sl(self: Arc<Self>) -> JoinHandle<Result<(), NRE>> {
            let join_handle = tokio::task::spawn(async {
                todo!();
            });
            join_handle
        }

        /// Start a _web server_ ("ws"): Accept WebSocket clients. Handle
        /// inbound command messages from authorized clients. Send state updates
        /// to authorized clients.
        pub fn start_ws(self: Arc<Self>) -> JoinHandle<Result<(), NRE>> {
            let join_handle = tokio::task::spawn(async {
                todo!();
            });
            join_handle
        }

        /// Start a _game server state machine_ ("gssm"): Loop game server
        /// state machine: Init -> Installing -> Launching -> RunningHealthy ->
        /// Stopping -> NotRunning -> Updating -> Launching -> RunningHealthy
        /// etc.
        ///
        /// Some of the transitions (like Init -> Installing -> Launching)
        /// should be automatically initiated at e.g. program startup, whereas
        /// some of the transitions should only happen upon received command
        /// from some authorized client (like RunningHealthy -> Stopping)
        pub fn start_gssm(self: Arc<Self>) -> JoinHandle<Result<(), NRE>> {
            let join_handle = tokio::task::spawn(async {
                todo!();
            });
            join_handle
        }

        /// Start a _system resources's usage monitor_ ("srum"): Read CPU,
        /// memory, networking usage etc., on a regular interval.
        pub fn start_srum(self: Arc<Self>) -> JoinHandle<Result<(), NRE>> {
            let join_handle = tokio::task::spawn(async {
                todo!();
            });
            join_handle
        }

        /// Start _game world snapshotting_ ("gws"): Query game world state via
        /// RCON, on a regular interval.
        pub fn start_gws(self: Arc<Self>) -> JoinHandle<Result<(), NRE>> {
            let join_handle = tokio::task::spawn(async {
                todo!();
            });
            join_handle
        }
    }
}
