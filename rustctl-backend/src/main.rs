/*
 * Rewrite in terms of the "actor pattern" (a concurrency pattern): There should
 * be _actors_ that own their stuff (such as I/O resources), and that perform
 * work in coroutines (alias "background tasks"), and that may communicate
 * with other actors via various _channels_. The main components of the
 * program should all be actors, and the program's main functionality should be
 * implemented by arranging channels between actors.
 *
 * More terminology:
 *
 * - "downstream WebSocket client": External web clients that connect to this
 *   program to e.g. receive state updates of the managed game server and to
 *   send command messages to be passed through via "upstream RCON WebSocket
 *   client"
 *
 * - "upstream RCON WebSocket client": Command interface of the managed game
 *   server.
 */
fn main() -> std::process::ExitCode {
    let cli_args: CliArgs = <CliArgs as clap::Parser>::parse();

    let _logger_handle: log4rs::Handle = init_logging(cli_args.log_level);
    log::info!(
        "{name} {version}",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    );

    /*
     * Drives (graceful) shutdown of the program upon specific signals.
     */
    let terminator: Terminator = Terminator::new();

    let config: Configuration = Configuration::resolve(cli_args.mock);

    let game_server_state_machine = match GameServerStateMachine::init(&config) {
        Ok(n) => n,
        Err(err) => {
            log::error!(
                "Failed to initialize game server state machine: {err_fmt}",
                err_fmt = fmt_source_tree(&err)
            );
            return std::process::ExitCode::FAILURE;
        }
    };

    let store = Store::init(config);

    let store_shared = std::sync::Arc::new(tokio::sync::Mutex::new(store));

    let game_ctl: GameServerController = GameServerController::new(&terminator);

    /*
     * Stage on which downstream WebSocket clients communicate.
     */
    let stage = Stage::new(&terminator, game_ctl.get_handle()); // "actors", hence "stage" :D

    /*
     * Accepts the downstream WebSocket connections.
     */
    let web_server = WebServer::new(&terminator, &stage);

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .enable_all()
        .build()
    {
        Ok(n) => n,
        Err(err) => {
            log::error!(
                "Failed to build async runtime: {err_fmt}",
                err_fmt = fmt_source_tree(&err)
            );
            return std::process::ExitCode::FAILURE;
        }
    };

    let runtime_done: (
        TerminatorSummary,
        WebServerSummary,
        StageSummary,
        GameServerControllerSummary,
    ) = runtime.block_on(async {
        let summary = tokio::join!(
            terminator.work(),
            web_server.work(),
            stage.work(),
            game_ctl.work(game_server_state_machine, store_shared.clone())
        );
        summary
    });

    let (status, ..) = runtime_done;
    let exit_status: std::process::ExitCode = (&status).into();

    exit_status
}

struct GameServerController {
    tx: tokio::sync::mpsc::Sender<DownstreamClientMessage>,
    rx: tokio::sync::mpsc::Receiver<DownstreamClientMessage>,
    summary: GameServerControllerSummary,
    cancel_read: tokio_util::sync::CancellationToken,
    cancel_write: tokio::sync::mpsc::Sender<()>,
}
impl GameServerController {
    fn new(terminator: &Terminator) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel::<DownstreamClientMessage>(1);

        let (cancel_write, cancel_read) = terminator.get_handle();

        Self {
            summary: GameServerControllerSummary,
            tx,
            rx,
            cancel_read,
            cancel_write,
        }
    }

    fn get_handle(&self) -> ActorHandle<DownstreamClientMessage> {
        ActorHandle::new(self.tx.clone())
    }

    async fn work(
        self,
        state_machine: GameServerStateMachine,
        store: std::sync::Arc<tokio::sync::Mutex<Store>>,
    ) -> GameServerControllerSummary {
        let token: tokio_util::sync::CancellationToken = self.cancel_read.child_token();
        let coroutine =
            tokio::spawn(async { state_machine.loop_transitions(token, self.rx, store).await });
        let done = self.cancel_read.run_until_cancelled(coroutine).await;
        if let Some(Ok(err)) = done {
            let err: NonRecoverableError = err;
            log::error!(
                "Game server controller failed: {err_fmt}",
                err_fmt = fmt_source_tree(&err)
            );
            match self.cancel_write.send(()).await {
                Ok(_) => log::debug!("Requested termination..."),
                Err(err) => log::error!(
                    "Failed to request termination: {err_fmt}",
                    err_fmt = fmt_source_tree(&err)
                ),
            }
        }
        self.summary
    }
}
struct GameServerControllerSummary;

#[allow(dead_code)] // TODO: Disallow dead code!
enum GameServerStateMachine {
    Init,
    Preparing,
    InstalledAndConfigured {
        cfg: Configuration,
    },
    Launching {
        process: tokio::process::Child,
        stdout: tokio::process::ChildStdout,
        stderr: tokio::process::ChildStderr,
    },
    RunningHealthy {
        process: tokio::process::Child,
    },
    SavingAndClosing,
    ClosedManually,
    TerminatedUnexpectedly,
}
impl GameServerStateMachine {
    pub fn init(cfg: &Configuration) -> Result<Self, NonRecoverableError> {
        if let Some(_pid) = is_process_running(cfg.get_installer_absolute()) {
            return Err(NonRecoverableError::ConcurrentGameServerInstaller);
        }

        if let Some(_pid) = is_process_running(cfg.get_game_absolute()) {
            return Err(NonRecoverableError::ConcurrentGameServer);
        }

        Ok(Self::Init)
    }

    pub async fn loop_transitions(
        mut self,
        cancellation_token: tokio_util::sync::CancellationToken,
        mut command_rx: tokio::sync::mpsc::Receiver<DownstreamClientMessage>,
        store: std::sync::Arc<tokio::sync::Mutex<Store>>,
    ) -> NonRecoverableError {
        loop {
            let state_before: String = self.to_string();
            match self {
                Self::Init => {
                    self = Self::Preparing;
                }

                /*
                 * Install or update `RustDedicated` using `steamcmd`.
                 */
                Self::Preparing => {
                    let cfg: Configuration;
                    {
                        let lock = store.lock().await;
                        cfg = lock.get_config().await;
                    }

                    let buildid_before: Option<u32> = {
                        if let Ok(contents) =
                            tokio::fs::read_to_string(cfg.get_manifest_absolute()).await
                        {
                            extract_buildid_from_buf(&contents)
                        } else {
                            None
                        }
                    };

                    let mut command = tokio::process::Command::new(cfg.get_installer_absolute());
                    command.current_dir(&cfg.root_dir_absolute);
                    command.args(cfg.get_installer_args());
                    command.stdout(std::process::Stdio::null());
                    command.stderr(std::process::Stdio::null());

                    let process: tokio::process::Child = match command.spawn() {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!(
                                "Failed to spawn game server installer ({path}): {err_fmt}",
                                path = cfg.get_installer_absolute().to_string_lossy(),
                                err_fmt = fmt_source_tree(&err),
                            );
                            return NonRecoverableError::CannotSpawnGameServerInstaller;
                        }
                    };

                    let _output: std::process::Output = process.wait_with_output().await.unwrap();

                    let buildid_after: Option<u32> = {
                        if let Ok(contents) =
                            tokio::fs::read_to_string(cfg.get_manifest_absolute()).await
                        {
                            extract_buildid_from_buf(&contents)
                        } else {
                            None
                        }
                    };

                    match (buildid_before, buildid_after) {
                        (_, None) => {
                            log::error!(
                                "Installing game server failed: Could not extract buildid from game server app manifest after installation: {path}",
                                path = cfg.get_manifest_absolute().to_string_lossy()
                            );
                        }
                        (None, Some(buildid)) => {
                            log::info!("Installed game server: buildid {buildid}");
                        }
                        (Some(buildid_before), Some(buildid_after)) => {
                            if buildid_before == buildid_after {
                                log::info!(
                                    "Installation checked: Game server is up to date: buildid {buildid_after}"
                                );
                            } else {
                                log::info!(
                                    "Updated game server: From buildid {buildid_before} to {buildid_after}"
                                );
                            }
                        }
                    }

                    self = Self::InstalledAndConfigured { cfg };
                }

                Self::InstalledAndConfigured { cfg } => {
                    let cfg: Configuration = cfg;
                    let mut command = tokio::process::Command::new(cfg.get_game_absolute());
                    command.current_dir(&cfg.root_dir_absolute);
                    command.args(cfg.get_game_args());
                    command.stdout(std::process::Stdio::piped());
                    command.stderr(std::process::Stdio::piped());

                    let mut process: tokio::process::Child = match command.spawn() {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!(
                                "Failed to spawn game server ({path}): {err_fmt}",
                                path = cfg.get_game_absolute().to_string_lossy(),
                                err_fmt = fmt_source_tree(&err),
                            );
                            return NonRecoverableError::CannotSpawnGameServer;
                        }
                    };

                    let stdout: tokio::process::ChildStdout = process.stdout.take().unwrap();
                    let stderr: tokio::process::ChildStderr = process.stderr.take().unwrap();

                    self = Self::Launching {
                        process,
                        stdout,
                        stderr,
                    };
                }

                Self::Launching {
                    process,
                    stdout,
                    stderr,
                } => {
                    let timeout = std::time::Duration::from_secs(60 * 30); // 30 minutes
                    let mut stdout_reader = tokio::io::BufReader::new(stdout);
                    let mut stderr_reader = tokio::io::BufReader::new(stderr);

                    // channel for signaling readiness from coroutine
                    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<()>();

                    let _read_stdout = tokio::spawn(async move {
                        let mut line = String::new();
                        let mut tx = Some(ready_tx);

                        loop {
                            line.clear();
                            match tokio::io::AsyncBufReadExt::read_line(
                                &mut stdout_reader,
                                &mut line,
                            )
                            .await
                            {
                                Ok(0) => {
                                    log::debug!("EOF reached: game server STDOUT");
                                    break;
                                }
                                Ok(_) => {
                                    let trimmed_line = line.trim_end();
                                    log::debug!(target: LOG_TARGET_GAME, "{trimmed_line}");
                                    if trimmed_line.contains("SteamServer Connected") {
                                        if let Some(sender) = tx.take() {
                                            let _ = sender.send(());
                                        }
                                    }
                                }
                                Err(err) => {
                                    log::error!(
                                        "Failed to read line from STDOUT: {err_fmt}",
                                        err_fmt = fmt_source_tree(&err)
                                    );
                                    break;
                                }
                            }
                        }
                    });

                    let _read_stderr = tokio::spawn(async move {
                        let mut line = String::new();

                        loop {
                            line.clear();
                            match tokio::io::AsyncBufReadExt::read_line(
                                &mut stderr_reader,
                                &mut line,
                            )
                            .await
                            {
                                Ok(0) => {
                                    log::debug!("EOF reached: game server STDERR");
                                    break;
                                }
                                Ok(_) => {
                                    let trimmed_line = line.trim_end();
                                    log::debug!(target: LOG_TARGET_GAME, "{trimmed_line}");
                                }
                                Err(err) => {
                                    log::error!(
                                        "Failed to read line from STDERR: {err_fmt}",
                                        err_fmt = fmt_source_tree(&err)
                                    );
                                    break;
                                }
                            }
                        }
                    });

                    let wait_readiness = async {
                        if let Err(err) = ready_rx.await {
                            let err: tokio::sync::oneshot::error::RecvError = err;
                            if !cancellation_token.is_cancelled() {
                                /*
                                 * The Err variant is expected when the channel gets teared
                                 * down, which is expected to happen when the program is
                                 * about to terminate, as indicated by the cancellation
                                 * token.
                                 *
                                 * If the Err variant is reached in any other scenario, then
                                 * that's a bug that should be investigated!
                                 */
                                todo!(
                                    "readiness channel receive failed while not cancelled: {err_fmt}",
                                    err_fmt = fmt_source_tree(&err)
                                );
                            }
                        }
                    };

                    match tokio::time::timeout(timeout, wait_readiness).await {
                        Ok(_) => {
                            self = Self::RunningHealthy { process };
                        }
                        Err(err) => {
                            log::error!(
                                "Game server did not indicate its readiness within timeout of {timeout_secs} seconds: {err_fmt}",
                                timeout_secs = timeout.as_secs(),
                                err_fmt = fmt_source_tree(&err)
                            );
                            return NonRecoverableError::GameServerStartupTimeout;
                        }
                    }
                }

                Self::RunningHealthy {
                    ref mut process, ..
                } => {
                    let event: GameCtlEvent = tokio::select! {
                        msg = command_rx.recv() => {
                            match msg {
                                Some(message) => GameCtlEvent::MessageReceived { message },
                                None => GameCtlEvent::MessageChannelClosed,
                            }
                        }
                        output = process.wait() => {
                            let exit_status: std::process::ExitStatus = output.unwrap();
                            GameCtlEvent::GameProcessTerminated { exit_status }
                        }
                    };

                    match event {
                        GameCtlEvent::MessageReceived { message } => {
                            let command: DownstreamClientMessage = message;
                            match command {
                                DownstreamClientMessage::ServerSaveAndClose => {
                                    /*
                                     * TODO: Issue SIGINT for the tracked game
                                     *       server child process, and transition to
                                     *       "SavingAndClosing".
                                     */
                                }
                                _ => {
                                    log::error!(
                                        "Ignoring unexpected command: {command:?} for current state: {self}"
                                    );
                                }
                            }
                        }
                        GameCtlEvent::MessageChannelClosed => todo!(),
                        GameCtlEvent::GameProcessTerminated { exit_status } => {
                            let _exit_status: std::process::ExitStatus = exit_status;
                            self = Self::TerminatedUnexpectedly;
                        }
                    }
                }

                Self::SavingAndClosing => {
                    /*
                     * TODO: Wait for the tracked child process to terminate and
                     *       be cleaned up. Check savefile of game world state
                     *       on disk. Then transition to "ClosedManually".
                     */
                }

                Self::ClosedManually => {
                    let msg = command_rx.recv().await;
                    if let Some(command) = msg {
                        let command: DownstreamClientMessage = command;
                        match command {
                            DownstreamClientMessage::ServerInstallOrUpdateAndStart => {
                                self = Self::Preparing;
                            }
                            _ => {
                                log::error!(
                                    "Ignoring unexpected command: {command:?} for current state: {self}"
                                );
                            }
                        }
                    }
                }

                Self::TerminatedUnexpectedly => {
                    self = Self::Preparing;
                }
            }
            let state_after: String = self.to_string();

            log::info!("Transitioned: {state_before} -> {state_after}");
        }
    }
}
impl std::fmt::Display for GameServerStateMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameServerStateMachine::Init => write!(f, "Init"),
            GameServerStateMachine::Preparing => write!(f, "Preparing"),
            GameServerStateMachine::InstalledAndConfigured { .. } => {
                write!(f, "InstalledAndConfigured")
            }
            GameServerStateMachine::Launching { .. } => write!(f, "Launching"),
            GameServerStateMachine::RunningHealthy { .. } => write!(f, "RunningHealthy"),
            GameServerStateMachine::SavingAndClosing => write!(f, "SavingAndClosing"),
            GameServerStateMachine::ClosedManually => write!(f, "ClosedManually"),
            GameServerStateMachine::TerminatedUnexpectedly => write!(f, "TerminatedUnexpectedly"),
        }
    }
}

#[allow(dead_code)] // TODO: Disallow dead code!
#[derive(Debug, Clone)]
struct Configuration {
    root_dir_absolute: std::path::PathBuf,
    installer_relative: std::path::PathBuf,
    game_relative: std::path::PathBuf,
    manifest_relative: std::path::PathBuf,

    game_world_size: u16,
    game_world_seed: u32,

    rcon_port: u16,
    rcon_password: String,
}
impl Configuration {
    pub fn resolve(mock: bool) -> Self {
        if !mock {
            todo!("only --mock mode is implemented for now");
        } else {
            let game_world_size = 1000;
            let game_world_seed = 1337;
            let rcon_port = 28016;
            let rcon_password = uuid::Uuid::new_v4().to_string();

            let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let crate_root_abs = crate_root.canonicalize().unwrap();
            let workspace_root_abs = crate_root_abs
                .parent()
                .expect("crate root should have parent")
                .to_path_buf();

            Self {
                root_dir_absolute: workspace_root_abs,
                installer_relative: std::path::Path::new(
                    "target/x86_64-unknown-linux-musl/debug/steamcmd",
                )
                .to_path_buf(),
                game_relative: std::path::Path::new(
                    "target/x86_64-unknown-linux-musl/debug/RustDedicated",
                )
                .to_path_buf(),
                manifest_relative: std::path::Path::new("mocks/dummy_manifest.acf").to_path_buf(),
                game_world_size,
                game_world_seed,
                rcon_port,
                rcon_password,
            }
        }
    }

    pub fn get_installer_absolute(&self) -> std::path::PathBuf {
        let mut path = self.root_dir_absolute.clone();
        path.push(&self.installer_relative);
        path
    }

    pub fn get_game_absolute(&self) -> std::path::PathBuf {
        let mut path = self.root_dir_absolute.clone();
        path.push(&self.game_relative);
        path
    }

    pub fn get_manifest_absolute(&self) -> std::path::PathBuf {
        let mut path = self.root_dir_absolute.clone();
        path.push(&self.manifest_relative);
        path
    }

    pub fn get_installer_args(&self) -> Vec<String> {
        vec![
            "+login".into(),
            "anonymous".into(),
            /*
             * WONTFIX: "force_install_dir" doesn't really "force" anything:
             *          Instead, SteamCMD seems to just create a new directory
             *          tree in "~/.local/share/Steam/" if it cannot access
             *          the given "force_install_dir". Therefore, we should
             *          add some checks to actually know where the installation
             *          ends up at. However, this is low priority as long as the
             *          specified directory is owned by the current user and so
             *          we can assume the command does what it's told to do.
             *
             *          Side note (opinionated): For SteamCMD, a more correct
             *          API would be to exit with failure status if a location
             *          that was requested "forced" cannot be used, and to NOT
             *          try to silently use some other location.
             *
             *          Behavior observed in `apt` packaged version:
             *          - Package: steamcmd:i386
             *          - Version: 0~20180105-5 (latest as of July 2025)
             *          - Section: non-free/games
             *          - Maintainer: Debian Games Team
             */
            "+force_install_dir".into(),
            self.root_dir_absolute.to_string_lossy().to_string(),
            "+app_update".into(),
            "258550".into(),
            "validate".into(),
            "+quit".into(),
        ]
    }

    pub fn get_game_args(&self) -> Vec<String> {
        vec![
            "-batchmode".into(),
            "+server.identity".into(),
            "instance0".into(),
            "+rcon.port".into(),
            self.rcon_port.to_string(),
            "+rcon.web".into(),
            "1".into(),
            "+rcon.password".into(),
            self.rcon_password.clone(),
        ]
    }
}

struct Terminator {
    summary: TerminatorSummary,
    cancellation_token: tokio_util::sync::CancellationToken,
    cancellation_channel: (
        tokio::sync::mpsc::Sender<()>,
        tokio::sync::mpsc::Receiver<()>,
    ),
}
impl Terminator {
    pub fn new() -> Self {
        Self {
            cancellation_token: tokio_util::sync::CancellationToken::new(),
            summary: TerminatorSummary(None),
            cancellation_channel: tokio::sync::mpsc::channel(1),
        }
    }

    pub async fn work(mut self) -> TerminatorSummary {
        let coroutine = tokio::spawn(async move {
            let mut sigint =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
            let exit_code: Option<std::process::ExitCode> = tokio::select! {
                _ = sigint.recv() => {
                    log::info!("SIGINT");
                    None
                },
                _ = sigterm.recv() => {
                    log::info!("SIGTERM");
                    None
                },
                _ = self.cancellation_channel.1.recv() => {
                    log::info!("Cancellation requested");
                    Some(std::process::ExitCode::FAILURE)
                },
            };
            self.cancellation_token.cancel();
            exit_code
        });

        let done = coroutine.await;
        if let Ok(Some(exit_code)) = done {
            self.summary = TerminatorSummary(Some(exit_code));
        }

        self.summary
    }

    pub fn get_handle(
        &self,
    ) -> (
        tokio::sync::mpsc::Sender<()>,
        tokio_util::sync::CancellationToken,
    ) {
        (
            self.cancellation_channel.0.clone(),
            self.cancellation_token.child_token(),
        )
    }
}

struct TerminatorSummary(Option<std::process::ExitCode>);
impl From<&TerminatorSummary> for std::process::ExitCode {
    fn from(value: &TerminatorSummary) -> Self {
        match value.0 {
            Some(exit_code) => exit_code,
            None => std::process::ExitCode::SUCCESS,
        }
    }
}

fn init_logging(level: log::LevelFilter) -> log4rs::Handle {
    const APPENDER_NAME_CORE: &str = "core";
    const APPENDER_NAME_GAME: &str = "game_server";

    let appender_core: log4rs::append::console::ConsoleAppender =
        log4rs::append::console::ConsoleAppender::builder()
            .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
                "{h({d(%Y-%m-%d %H:%M:%S)(utc)} [rustctl] {m})} [{f}:{L}]\n",
            )))
            .build();

    let appender_game: log4rs::append::console::ConsoleAppender =
        log4rs::append::console::ConsoleAppender::builder()
            .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
                "{h({d(%Y-%m-%d %H:%M:%S)(utc)} [{t}] {m})}\n",
            )))
            .build();

    let appender_cfg_core: log4rs::config::Appender =
        log4rs::config::Appender::builder().build(APPENDER_NAME_CORE, Box::new(appender_core));

    let appender_cfg_game: log4rs::config::Appender =
        log4rs::config::Appender::builder().build(APPENDER_NAME_GAME, Box::new(appender_game));

    let config = log4rs::Config::builder()
        .appender(appender_cfg_core)
        .appender(appender_cfg_game)
        .logger(
            log4rs::config::Logger::builder()
                .appender(APPENDER_NAME_GAME)
                .additive(false) // log only for the specific target, i.e. don't propagate duplicate log
                .build(LOG_TARGET_GAME, level),
        )
        .build(
            log4rs::config::Root::builder()
                .appender(APPENDER_NAME_CORE)
                .build(level),
        )
        .unwrap();

    log4rs::init_config(config).unwrap()
}

#[derive(clap::Parser, Debug)]
#[command(version)]
pub struct CliArgs {
    #[arg(short, long, default_value_t = log::LevelFilter::Debug)]
    pub log_level: log::LevelFilter,

    #[arg(long, default_value_t = false)]
    pub mock: bool,
}

#[derive(Clone)]
struct WebServerState {
    stage: ActorHandle<DownstreamClientMessage>,
    clients_connected:
        std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<uuid::Uuid, DownstreamClient>>>,
}

impl WebServerState {
    pub fn new(stage: ActorHandle<DownstreamClientMessage>) -> Self {
        Self {
            stage,
            clients_connected: std::sync::Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
        }
    }

    pub fn get_stage_handle(&self) -> ActorHandle<DownstreamClientMessage> {
        self.stage.clone()
    }

    pub async fn register_client(&mut self, client: DownstreamClient) -> (uuid::Uuid, usize) {
        let id = uuid::Uuid::new_v4();
        let connected_total: usize;

        {
            let mut lock = self.clients_connected.lock().await;
            lock.insert(id, client);
            connected_total = lock.len();
        }

        (id, connected_total)
    }

    pub async fn unregister_client(&mut self, id: &uuid::Uuid) -> usize {
        let connected_remaining: usize;

        {
            let mut lock = self.clients_connected.lock().await;
            lock.remove(id);
            connected_remaining = lock.len();
        }

        connected_remaining
    }
}

struct WebServer {
    summary: WebServerSummary,
    cancel_read: tokio_util::sync::CancellationToken,
    router: axum::Router,
}
impl WebServer {
    pub fn new(terminator: &Terminator, stage: &Stage) -> Self {
        let state = WebServerState::new(stage.get_handle());

        let router: axum::Router = axum::Router::new()
            .route("/ws", axum::routing::get(websocket_handler))
            .with_state(state);

        Self {
            summary: WebServerSummary {},
            cancel_read: terminator.get_handle().1,
            router,
        }
    }

    pub async fn work(self) -> WebServerSummary {
        let tcp_listener = match tokio::net::TcpListener::bind("127.0.0.1:8080").await {
            Ok(n) => n,
            Err(err) => {
                log::error!(
                    "Failed to bind TCP listener: {err_fmt}",
                    err_fmt = fmt_source_tree(&err)
                );
                return self.summary;
            }
        };

        let service = self
            .router
            .into_make_service_with_connect_info::<std::net::SocketAddr>();

        let serve = axum::serve(tcp_listener, service);

        if let Some(Err(err)) = self
            .cancel_read
            .run_until_cancelled(async move { serve.await })
            .await
        {
            /*
             * From docs (axum v0.8.4):
             *   fn axum::serve "will never actually complete or return an error"
             */
            unreachable!("{err}")
        }
        self.summary
    }
}
struct WebServerSummary;

async fn websocket_handler(
    ws: axum::extract::WebSocketUpgrade,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    axum::extract::State(state): axum::extract::State<WebServerState>,
) -> axum::response::Response {
    ws.on_upgrade(async move |socket| {
        let socket: axum::extract::ws::WebSocket = socket;
        let addr: std::net::SocketAddr = addr;
        let mut state: WebServerState = state;

        let client = DownstreamClient::new();
        let (client_id, connected_total) = state.register_client(client).await;
        log::info!("Downstream client connected: {addr} -- {connected_total} connected clients in total");

        let (tx, rx) = futures_util::StreamExt::split(socket);
        let mut sender = DownstreamClientSender::new(tx);
        let mut receiver = DownstreamClientReceiver::new(rx);

        let _done: () = tokio::select!(
            done = sender.work() => done,
            done = receiver.work(state.get_stage_handle()) => done,
        );

        let connected_remaining = state.unregister_client(&client_id).await;
        log::info!(
            "Downstream client disconnected: {addr} -- {connected_remaining} connected clients remain"
        );
    })
}

#[derive(Clone)]
struct DownstreamClient {}

impl DownstreamClient {
    pub fn new() -> Self {
        Self {}
    }
}

struct DownstreamClientSender {
    tx: futures_util::stream::SplitSink<axum::extract::ws::WebSocket, axum::extract::ws::Message>,
}

impl DownstreamClientSender {
    pub fn new(
        tx: futures_util::stream::SplitSink<
            axum::extract::ws::WebSocket,
            axum::extract::ws::Message,
        >,
    ) -> Self {
        Self { tx }
    }

    /*
     * TODO: Instead of mock ticking with interval, continuosly receive from
     *       other actors: Send state updates as fast as the other actors
     *       produce them! I.e. the other actors being some mechanisms that
     *       query in-game state updates (via RCON), or system resources usage
     *       snapshots etc.
     */
    pub async fn work(&mut self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        'send_messages: loop {
            interval.tick().await;
            let send = futures_util::SinkExt::send(
                &mut self.tx,
                "Hello to downstream client from server!".into(),
            );
            if let Err(err) = send.await {
                let err: axum::Error = err;
                log::error!(
                    "Failed to send message to downstream client: {err_fmt}",
                    err_fmt = fmt_source_tree(&err)
                );
                break 'send_messages;
            }
        }
    }
}

struct DownstreamClientReceiver {
    rx: futures_util::stream::SplitStream<axum::extract::ws::WebSocket>,
}

impl DownstreamClientReceiver {
    pub fn new(rx: futures_util::stream::SplitStream<axum::extract::ws::WebSocket>) -> Self {
        Self { rx }
    }

    pub async fn work(&mut self, stage: ActorHandle<DownstreamClientMessage>) {
        'recv_messages: loop {
            let next = futures_util::StreamExt::next(&mut self.rx);
            let msg: axum::extract::ws::Message = match next.await {
                Some(Ok(n)) => n,
                Some(Err(err)) => {
                    /*
                     * Client closing the connection non-gracefully is not the
                     * happy path, but there might also not be anything we can
                     * do about it (e.g. in the case of the client's networking
                     * device just exploding or something), therefore logging
                     * as warning.
                     */
                    log::warn!(
                        "Client likely closed non-gracefully: Failed to receive message from downstream client: {err_fmt}",
                        err_fmt = fmt_source_tree(&err)
                    );
                    break 'recv_messages;
                }
                None => {
                    break 'recv_messages;
                }
            };

            /*
             * TODO: Implement graceful disconnect: Unregister the client when
             *       graceful close message is received!
             */
            let msg: DownstreamClientMessage = match (&msg).try_into() {
                Ok(n) => n,
                Err(err) => {
                    /*
                     * Client misbehavior indicates a bug in the client, in
                     * which case we should drop it.
                     */
                    log::error!(
                        "Received invalid message from a downstream client: {err_fmt} -- Stopping handling!",
                        err_fmt = fmt_source_tree(&err)
                    );
                    break 'recv_messages;
                }
            };

            if let Err(err) = stage.send(msg).await {
                log::error!(
                    "Failed to send downstream client message to stage: {err_fmt}",
                    err_fmt = fmt_source_tree(&err)
                );
                /*
                 * Not being able to send a message to stage may indicate a
                 * non-recoverable error case! (In case e.g. out of memory or
                 * something...)
                 */
                todo!("request termination of the program");
            }
        }
    }
}

#[allow(dead_code)] // TODO: Disallow dead code!
#[derive(Clone, Debug, serde::Deserialize)]
enum DownstreamClientMessage {
    ServerSaveAndClose,
    ServerConfigure { cfg: GameServerConfigurationPatch },
    ServerInstallOrUpdateAndStart,
    GameWorldKillPlayer { id: String },
    WebSocketProtocolOther,
}
#[derive(Clone, Debug, serde::Deserialize)]
struct GameServerConfigurationPatch {}

impl TryFrom<&axum::extract::ws::Message> for DownstreamClientMessage {
    type Error = serde_json::Error;

    fn try_from(value: &axum::extract::ws::Message) -> Result<Self, Self::Error> {
        let utf8: String = match value {
            axum::extract::ws::Message::Text(utf8_bytes) => utf8_bytes.to_string(),
            axum::extract::ws::Message::Binary(_)
            | axum::extract::ws::Message::Ping(_)
            | axum::extract::ws::Message::Pong(_)
            | axum::extract::ws::Message::Close(_) => return Ok(Self::WebSocketProtocolOther),
        };
        let message: DownstreamClientMessage = serde_json::from_str(&utf8)?;
        Ok(message)
    }
}

struct Stage {
    summary: StageSummary,
    cancel_read: tokio_util::sync::CancellationToken,
    channel: (
        tokio::sync::mpsc::Sender<DownstreamClientMessage>,
        tokio::sync::mpsc::Receiver<DownstreamClientMessage>,
    ),
    game_ctl: ActorHandle<DownstreamClientMessage>,
}
impl Stage {
    fn new(terminator: &Terminator, game_ctl: ActorHandle<DownstreamClientMessage>) -> Self {
        Self {
            channel: tokio::sync::mpsc::channel(1),
            cancel_read: terminator.get_handle().1,
            summary: StageSummary { messages_total: 0 },
            game_ctl,
        }
    }

    fn get_handle(&self) -> ActorHandle<DownstreamClientMessage> {
        let (tx, _rx) = &self.channel;
        ActorHandle::new(tx.clone())
    }

    async fn work(mut self) -> StageSummary {
        let (_tx, mut rx) = self.channel;
        let coroutine = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Some(no_overflow) = self.summary.messages_total.checked_add(1) {
                    self.summary.messages_total = no_overflow;
                }
                let msg: DownstreamClientMessage = msg;
                if let Err(err) = self.game_ctl.send(msg).await {
                    log::error!(
                        "Failed to send downstream client message from stage to game server controller: {err_fmt}",
                        err_fmt = fmt_source_tree(&err)
                    );
                }
            }
        });
        _ = self.cancel_read.run_until_cancelled(coroutine).await;
        self.summary
    }
}
struct StageSummary {
    messages_total: u128,
}

#[derive(Clone)]
struct ActorHandle<Message> {
    tx: tokio::sync::mpsc::Sender<Message>,
}
impl<Message> ActorHandle<Message> {
    pub fn new(tx: tokio::sync::mpsc::Sender<Message>) -> Self {
        Self { tx }
    }

    pub async fn send(
        &self,
        message: Message,
    ) -> Result<(), tokio::sync::mpsc::error::TrySendError<Message>> {
        self.tx.try_send(message)
    }
}

struct Store {
    in_mem: Configuration,
}
impl Store {
    pub fn init(initial_config: Configuration) -> Self {
        Self {
            in_mem: initial_config,
        }
    }

    pub async fn get_config(&self) -> Configuration {
        self.in_mem.clone()
    }
}

fn extract_buildid_from_buf(buf: &str) -> Option<u32> {
    let vdf: keyvalues_parser::Vdf = match keyvalues_parser::Vdf::parse(buf) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let root: &keyvalues_parser::Obj = vdf.value.get_obj()?;

    let buildid_str: &str = match root.get("buildid") {
        Some(values) => {
            if values.len() != 1 {
                return None;
            }
            values[0].get_str()?
        }
        None => return None,
    };

    buildid_str.parse::<u32>().ok()
}

#[derive(Debug)]
enum NonRecoverableError {
    /// Game server installer is running when it is not expected to be.
    ConcurrentGameServerInstaller,

    /// Game server is running when it is not expected to be.
    ConcurrentGameServer,

    /// Cannot spawn `steamcmd`.
    CannotSpawnGameServerInstaller,

    /// Cannot spawn `RustDedicated`.
    CannotSpawnGameServer,

    /// Launched game server did not pass health check within timeout.
    GameServerStartupTimeout,
}

impl std::error::Error for NonRecoverableError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            NonRecoverableError::ConcurrentGameServerInstaller => None,
            NonRecoverableError::ConcurrentGameServer => None,
            NonRecoverableError::CannotSpawnGameServerInstaller => None,
            NonRecoverableError::CannotSpawnGameServer => None,
            NonRecoverableError::GameServerStartupTimeout => None,
        }
    }
}

impl std::fmt::Display for NonRecoverableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NonRecoverableError::ConcurrentGameServerInstaller => {
                write!(f, "game server installer running already")
            }
            NonRecoverableError::CannotSpawnGameServerInstaller => {
                write!(f, "cannot spawn process for game server installer")
            }
            NonRecoverableError::ConcurrentGameServer => {
                write!(f, "game server running already")
            }
            NonRecoverableError::CannotSpawnGameServer => {
                write!(f, "cannot spawn process for game server")
            }
            NonRecoverableError::GameServerStartupTimeout => {
                write!(f, "game server startup timeout")
            }
        }
    }
}

const LOG_TARGET_GAME: &str = "game";

enum GameCtlEvent {
    MessageReceived {
        message: DownstreamClientMessage,
    },

    MessageChannelClosed,

    GameProcessTerminated {
        exit_status: std::process::ExitStatus,
    },
}

/// Like `pgrep`: Check if there's a program with given name running. Returns
/// the running process's ID (PID) if so.
fn is_process_running(executable: std::path::PathBuf) -> Option<u32> {
    let name = match executable.file_name() {
        Some(n) => n,
        None => {
            return None;
        }
    };

    let proc_dir = match std::fs::read_dir("/proc") {
        Ok(dir) => dir,
        Err(_) => return None,
    };

    let seekable: &str = name.to_str()?;

    for entry in proc_dir.flatten() {
        let item: std::ffi::OsString = entry.file_name();
        let item: &str = item.to_str()?;

        if item.chars().all(|c| c.is_ascii_digit()) {
            let pid_path = entry.path();

            // comm file: contains the process name
            let comm = pid_path.join("comm");
            if let Ok(buf) = std::fs::read_to_string(&comm) {
                let process_name: &str = buf.trim();
                if process_name == seekable {
                    if let Ok(pid) = item.parse::<u32>() {
                        return Some(pid);
                    }
                }
            }
        }
    }

    None
}

fn fmt_source_tree<E>(error: &E) -> String
where
    E: std::error::Error,
{
    let mut concatenated: String = String::new();

    concatenated.push_str(&format!("{error}"));

    let mut source: Option<_> = error.source();
    while let Some(src) = source {
        concatenated.push_str(&format!(": {src}"));
        source = src.source();
    }

    concatenated
}
