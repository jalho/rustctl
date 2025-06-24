fn main() -> std::process::ExitCode {
    let _handle = logging::init_logging(log::LevelFilter::Trace);
    log::info!(
        "{name} {version}",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    );

    let coordinator: std::sync::Arc<coord::Coordinator> = coord::Coordinator::init();
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let (cancel_tx, cancel_rx) = tokio::sync::mpsc::channel::<()>(1);

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
            coordinator
                .clone()
                .start_srum(cancellation_token.child_token(), cancel_tx.clone()),
            coordinator
                .clone()
                .start_gws(cancellation_token.child_token(), cancel_tx.clone()),
            coordinator
                .clone()
                .start_ws(cancellation_token.child_token(), cancel_tx.clone()),
            coordinator.clone().start_sl(cancellation_token, cancel_rx),
        )
    });

    let code: std::process::ExitCode = coord::CoroutinesTerminated::capture(coroutines_done).into();
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

mod coord {
    // TODO: Disallow dead_code
    #[allow(dead_code)]
    pub struct Coordinator {
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

        /// Start _signal listener_ ("sl"): Activate the CancellationToken on
        /// SIGINT, SIGTERM, or whenever any of the peer coroutines use the
        /// `mpsc::channel` to signal to terminate.
        pub fn start_sl(
            self: std::sync::Arc<Self>,
            cancellation_token: tokio_util::sync::CancellationToken,
            mut cancel_rx: tokio::sync::mpsc::Receiver<()>,
        ) -> tokio::task::JoinHandle<Result<(), crate::error::NRE>> {
            let join_handle = tokio::task::spawn(async move {
                let mut sigint =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                        .unwrap();
                let mut sigterm =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .unwrap();
                tokio::select! {
                    _ = sigint.recv() => log::info!("SIGINT"),
                    _ = sigterm.recv() => log::info!("SIGTERM"),
                    _ = cancel_rx.recv() => log::info!("Shutdown requested by coroutine"),
                }
                cancellation_token.cancel();
                Ok(())
            });
            join_handle
        }

        /// Start a _web server_ ("ws"): Accept WebSocket clients. Handle
        /// inbound command messages from authorized clients. Send state updates
        /// to authorized clients.
        pub fn start_ws(
            self: std::sync::Arc<Self>,
            _cancellation_token: tokio_util::sync::CancellationToken,
            _cancel_tx: tokio::sync::mpsc::Sender<()>,
        ) -> tokio::task::JoinHandle<Result<(), crate::error::NRE>> {
            let join_handle = tokio::task::spawn(async {
                todo!();
            });
            join_handle
        }

        /// Start a _system resources's usage monitor_ ("srum"): Read CPU,
        /// memory, networking usage etc., on a regular interval.
        pub fn start_srum(
            self: std::sync::Arc<Self>,
            cancellation_token: tokio_util::sync::CancellationToken,
            _cancel_tx: tokio::sync::mpsc::Sender<()>,
        ) -> tokio::task::JoinHandle<Result<(), crate::error::NRE>> {
            let join_handle = tokio::task::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
                loop {
                    interval.tick().await;
                    let mut srum = self.system_resources_usage.lock().await;
                    srum.read_cpu().await;
                    if cancellation_token.is_cancelled() {
                        break;
                    }
                }
                Ok(())
            });
            join_handle
        }

        /// Start _game world snapshotting_ ("gws"): Query game world state via
        /// RCON, on a regular interval.
        pub fn start_gws(
            self: std::sync::Arc<Self>,
            _cancellation_token: tokio_util::sync::CancellationToken,
            _cancel_tx: tokio::sync::mpsc::Sender<()>,
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
            ),
        ) -> Self {
            let (a, b, c, d) = results;
            Self {
                results: vec![a, b, c, d],
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
    pub mod system_resources_usage {
        pub struct SystemResourcesUsage {
            cpu_usage: u8,
            memory_usage: u8,
        }

        impl SystemResourcesUsage {
            pub fn init() -> Self {
                Self {
                    cpu_usage: 0,
                    memory_usage: 0,
                }
            }

            pub async fn read_cpu(&mut self) {
                self.cpu_usage = self.cpu_usage + 1;
            }
        }
    }

    pub mod web_server {
        pub struct WebClientsConnected {
            clients_connected: std::collections::HashMap<std::net::SocketAddr, ()>,
        }

        impl WebClientsConnected {
            pub fn init() -> Self {
                Self {
                    clients_connected: std::collections::HashMap::new(),
                }
            }
        }
    }

    pub mod game_server_state_machine {
        pub struct NotRunning;

        pub enum GameServerStateMachine {
            NotRunning(NotRunning),
        }

        impl GameServerStateMachine {
            pub fn init() -> Self {
                Self::NotRunning(NotRunning {})
            }
        }
    }

    pub mod game_world_snapshotting {
        pub struct GameWorldSnapshot {
            players: std::collections::HashMap<String, ()>,
        }

        impl GameWorldSnapshot {
            pub fn init() -> Self {
                Self {
                    players: std::collections::HashMap::new(),
                }
            }
        }
    }
}
