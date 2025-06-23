fn main() -> std::process::ExitCode {
    let coordinator: std::sync::Arc<coord::Coordinator> = coord::Coordinator::init();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    /*
     * TODO: Define coroutine for receiving events from game server via a Unix
     *       domain socket (i.e. those sent from a custom made Carbon framework
     *       plugin)
     */
    let coroutines_done = runtime.block_on(async {
        tokio::join!(
            coordinator.clone().start_sl(),
            coordinator.clone().start_srum(),
            coordinator.clone().start_gssm(),
            coordinator.clone().start_gws(),
            coordinator.clone().start_ws(),
        )
    });

    let code: std::process::ExitCode = coord::CoroutinesTerminated::capture(coroutines_done).into();
    return code;
}

mod coord {
    // TODO: Disallow dead_code
    #[allow(dead_code)]
    pub struct Coordinator {
        cancellation_token: tokio_util::sync::CancellationToken,
        cancellation_channel: (
            tokio::sync::mpsc::Sender<crate::coroutines::Coroutine>,
            tokio::sync::mpsc::Receiver<crate::coroutines::Coroutine>,
        ),

        system_resources_usage:
            tokio::sync::Mutex<crate::coroutines::system_resources_usage::SystemResourcesUsage>,
        web_clients_connected:
            tokio::sync::Mutex<crate::coroutines::web_server::WebClientsConnected>,
        game_server_state_machine: tokio::sync::Mutex<
            crate::coroutines::game_server_state_machine::GameServerStateMachine,
        >,
        game_world_snapshot:
            tokio::sync::Mutex<crate::coroutines::game_world_snapshotting::GameWorldSnapshot>,
    }

    impl Coordinator {
        pub fn init() -> std::sync::Arc<Self> {
            std::sync::Arc::new(Self {
                cancellation_token: tokio_util::sync::CancellationToken::new(),
                cancellation_channel: tokio::sync::mpsc::channel::<crate::coroutines::Coroutine>(1),

                system_resources_usage: tokio::sync::Mutex::new(
                    crate::coroutines::system_resources_usage::SystemResourcesUsage::init(),
                ),
                web_clients_connected: tokio::sync::Mutex::new(
                    crate::coroutines::web_server::WebClientsConnected::init(),
                ),
                game_server_state_machine: tokio::sync::Mutex::new(
                    crate::coroutines::game_server_state_machine::GameServerStateMachine::init(),
                ),
                game_world_snapshot: tokio::sync::Mutex::new(
                    crate::coroutines::game_world_snapshotting::GameWorldSnapshot::init(),
                ),
            })
        }

        /// Start _signal listener_ ("sl"): Activate the CancellationToken in
        /// `self` on SIGINT, SIGTERM, or whenever any of the peer coroutines
        /// use the `mpsc::channel` in `self` to signal to terminate.
        pub fn start_sl(
            self: std::sync::Arc<Self>,
        ) -> tokio::task::JoinHandle<Result<(), crate::error::NRE>> {
            let join_handle = tokio::task::spawn(async {
                todo!();
            });
            join_handle
        }

        /// Start a _web server_ ("ws"): Accept WebSocket clients. Handle
        /// inbound command messages from authorized clients. Send state updates
        /// to authorized clients.
        pub fn start_ws(
            self: std::sync::Arc<Self>,
        ) -> tokio::task::JoinHandle<Result<(), crate::error::NRE>> {
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
        pub fn start_gssm(
            self: std::sync::Arc<Self>,
        ) -> tokio::task::JoinHandle<Result<(), crate::error::NRE>> {
            let join_handle = tokio::task::spawn(async {
                /*
                 * TODO: Maybe `start_gssm` is redundant? The automatic
                 *       transition (i.e. Init -> Installing -> Launching etc.)
                 *       could initiate at Coordinator::init, and any subsequent
                 *       transitions could be initiated upon received commands
                 *       from authorized clients, i.e. in `start_ws`?
                 */
                todo!();
            });
            join_handle
        }

        /// Start a _system resources's usage monitor_ ("srum"): Read CPU,
        /// memory, networking usage etc., on a regular interval.
        pub fn start_srum(
            self: std::sync::Arc<Self>,
        ) -> tokio::task::JoinHandle<Result<(), crate::error::NRE>> {
            let join_handle = tokio::task::spawn(async {
                todo!();
            });
            join_handle
        }

        /// Start _game world snapshotting_ ("gws"): Query game world state via
        /// RCON, on a regular interval.
        pub fn start_gws(
            self: std::sync::Arc<Self>,
        ) -> tokio::task::JoinHandle<Result<(), crate::error::NRE>> {
            let join_handle = tokio::task::spawn(async {
                todo!();
            });
            join_handle
        }
    }

    pub struct CoroutinesTerminated {
        results: Vec<Result<Result<(), crate::error::NRE>, tokio::task::JoinError>>,
    }

    impl CoroutinesTerminated {
        pub fn capture(
            results: (
                Result<Result<(), crate::error::NRE>, tokio::task::JoinError>,
                Result<Result<(), crate::error::NRE>, tokio::task::JoinError>,
                Result<Result<(), crate::error::NRE>, tokio::task::JoinError>,
                Result<Result<(), crate::error::NRE>, tokio::task::JoinError>,
                Result<Result<(), crate::error::NRE>, tokio::task::JoinError>,
            ),
        ) -> Self {
            let (a, b, c, d, e) = results;
            Self {
                results: vec![a, b, c, d, e],
            }
        }
    }

    impl From<CoroutinesTerminated> for std::process::ExitCode {
        fn from(value: CoroutinesTerminated) -> Self {
            'results: for result in value.results {
                match result {
                    Ok(Ok(ok)) => {
                        let _coroutine_ok: () = ok;
                        continue 'results;
                    }
                    Ok(Err(err)) => {
                        let _err: crate::error::NRE = err;
                        return std::process::ExitCode::FAILURE;
                    }
                    Err(err) => {
                        let _err: tokio::task::JoinError = err;
                        return std::process::ExitCode::FAILURE;
                    }
                }
            }
            return std::process::ExitCode::SUCCESS;
        }
    }
}

mod error {
    /// A _non-recoverable error_ (NRE).
    #[derive(Debug)]
    pub enum NRE {
        MissingRequiredDependency,
    }
}

mod coroutines {

    // TODO: Disallow dead_code
    #[allow(dead_code)]
    pub enum Coroutine {
        SystemResourcesUsage(system_resources_usage::SystemResourcesUsage),
        WebClientsConnected(web_server::WebClientsConnected),
        GameServerStateMachine(game_server_state_machine::GameServerStateMachine),
        GameWorldSnapshot(game_world_snapshotting::GameWorldSnapshot),
    }

    pub mod system_resources_usage {
        pub struct SystemResourcesUsage;

        impl SystemResourcesUsage {
            pub fn init() -> Self {
                todo!()
            }
        }
    }

    pub mod web_server {
        pub struct WebClientsConnected;

        impl WebClientsConnected {
            pub fn init() -> Self {
                todo!()
            }
        }
    }

    pub mod game_server_state_machine {
        pub enum GameServerStateMachine {}

        impl GameServerStateMachine {
            pub fn init() -> Self {
                todo!()
            }
        }
    }

    pub mod game_world_snapshotting {
        pub struct GameWorldSnapshot;

        impl GameWorldSnapshot {
            pub fn init() -> Self {
                todo!()
            }
        }
    }
}
