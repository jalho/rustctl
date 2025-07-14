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

    let store: std::sync::Arc<tokio::sync::Mutex<Store>> = Store::new(&cli_args);

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
            log::error!("failed to build async runtime: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let _runtime_done: (
        TerminatorSummary,
        WebServerSummary,
        StageSummary,
        GameServerControllerSummary,
    ) = runtime.block_on(async {
        let summary = tokio::join!(
            terminator.work(),
            web_server.work(),
            stage.work(),
            game_ctl.work(store.clone())
        );
        return summary;
    });

    std::process::ExitCode::SUCCESS
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
        store: std::sync::Arc<tokio::sync::Mutex<Store>>,
    ) -> GameServerControllerSummary {
        let coroutine = tokio::spawn(async {
            let done = GameServerStateMachine::init()
                .loop_transitions(self.rx, store)
                .await;
            return done;
        });
        let done = self.cancel_read.run_until_cancelled(coroutine).await;
        if let Some(Ok(err)) = done {
            let _err: std::io::Error = err;
            log::debug!("Requesting cancellation...");
            self.cancel_write.send(()).await.unwrap();
        }
        return self.summary;
    }
}
struct GameServerControllerSummary;

enum GameServerStateMachine {
    Init,
    Preparing,
    InstalledAndConfigured { cfg: Configuration },
    Launching,
    RunningHealthy,
    SavingAndClosing,
    ClosedManually,
    TerminatedUnexpectedly,
}
impl GameServerStateMachine {
    pub fn init() -> Self {
        Self::Init
    }

    pub async fn loop_transitions(
        mut self,
        mut command_rx: tokio::sync::mpsc::Receiver<DownstreamClientMessage>,
        store: std::sync::Arc<tokio::sync::Mutex<Store>>,
    ) -> std::io::Error {
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
                                "failed to spawn game server installer ({path}): {err}",
                                path = cfg.get_installer_absolute().to_string_lossy()
                            );
                            return err;
                        }
                    };

                    let output: std::process::Output = match process.wait_with_output().await {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!(
                                "game server installer ({path}) failed: {err}",
                                path = cfg.get_installer_absolute().to_string_lossy()
                            );
                            return err;
                        }
                    };

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
                                "failed to extract buildid from game server app manifest after installation: {path}",
                                path = cfg.get_manifest_absolute().to_string_lossy()
                            );
                        }
                        (None, Some(buildid)) => {
                            log::info!("game server installed: buildid {buildid}");
                        }
                        (Some(buildid_before), Some(buildid_after)) => {
                            if buildid_before == buildid_after {
                                log::info!(
                                    "installed game server is up to date: buildid {buildid_after}"
                                );
                            } else {
                                log::info!(
                                    "game server updated from buildid {buildid_before} to buildid {buildid_after}"
                                );
                            }
                        }
                    }

                    self = Self::InstalledAndConfigured { cfg };
                }

                Self::InstalledAndConfigured { cfg } => {
                    let cfg: Configuration = cfg;
                    /*
                     * TODO: Spawn the game server and track the spawned child
                     *       process.
                     */
                    self = Self::Launching;
                }

                Self::Launching => {
                    /*
                     * TODO: Wait for the tracked child process to emit to
                     *       stdout: "SteamServer Connected". Then, transition
                     *       to "RunningHealthy".
                     */
                    self = Self::RunningHealthy;
                }

                Self::RunningHealthy => {
                    let event = tokio::select! {
                        cmd = command_rx.recv() => {
                            cmd
                        }
                        /*
                         * TODO: Add branch for case tracked child process
                         *       terminated unexpectedly. From there, we should
                         *       transition to "TerminatedUnexpectedly".
                         */
                    };
                    if let Some(command) = event {
                        let command: DownstreamClientMessage = command;
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
                                    "ignoring unexpected command: {command:?} for current state: {self}"
                                );
                            }
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
                                    "ignoring unexpected command: {command:?} for current state: {self}"
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

            log::info!("Transitioned from {state_before} to {state_after}");
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
            GameServerStateMachine::Launching => write!(f, "Launching"),
            GameServerStateMachine::RunningHealthy => write!(f, "RunningHealthy"),
            GameServerStateMachine::SavingAndClosing => write!(f, "SavingAndClosing"),
            GameServerStateMachine::ClosedManually => write!(f, "ClosedManually"),
            GameServerStateMachine::TerminatedUnexpectedly => write!(f, "TerminatedUnexpectedly"),
        }
    }
}

#[derive(Debug, Clone)]
struct Configuration {
    root_dir_absolute: std::path::PathBuf,
    installer_relative: std::path::PathBuf,
    game_relative: std::path::PathBuf,
    manifest_relative: std::path::PathBuf,

    game_world_size: u16,
    game_world_seed: u32,
}
impl Configuration {
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
            summary: TerminatorSummary {},
            cancellation_channel: tokio::sync::mpsc::channel(1),
        }
    }

    pub async fn work(mut self) -> TerminatorSummary {
        let coroutine = tokio::spawn(async move {
            let mut sigint =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
            tokio::select! {
                _ = sigint.recv() => log::info!("SIGINT"),
                _ = sigterm.recv() => log::info!("SIGTERM"),
                _ = self.cancellation_channel.1.recv() => log::info!("Cancellation requested"),
            }
            self.cancellation_token.cancel();
        });
        _ = coroutine.await;
        self.summary
    }

    pub fn get_handle(
        &self,
    ) -> (
        tokio::sync::mpsc::Sender<()>,
        tokio_util::sync::CancellationToken,
    ) {
        return (
            self.cancellation_channel.0.clone(),
            self.cancellation_token.child_token(),
        );
    }
}
struct TerminatorSummary;

fn init_logging(level: log::LevelFilter) -> log4rs::Handle {
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

#[derive(clap::Parser, Debug)]
#[command(version)]
pub struct CliArgs {
    #[arg(short, long, default_value_t = log::LevelFilter::Debug)]
    pub log_level: log::LevelFilter,

    #[arg(long, default_value_t = false)]
    pub mock: bool,
}

struct WebServer {
    summary: WebServerSummary,
    cancel_read: tokio_util::sync::CancellationToken,
    router: axum::Router,
}
impl WebServer {
    pub fn new(terminator: &Terminator, stage: &Stage) -> Self {
        let router: axum::Router = axum::Router::new()
            .route("/ws", axum::routing::get(websocket_handler))
            .with_state(stage.get_handle());

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
                log::error!("failed to bind TCP listener: {err}");
                return self.summary;
            }
        };
        if let Some(Err(err)) = self
            .cancel_read
            .run_until_cancelled(async move {
                /*
                 * From docs (axum v0.8.4):
                 *   "will never actually complete or return an error"
                 */
                axum::serve(tcp_listener, self.router).await
            })
            .await
        {
            unreachable!("{err}")
        }
        self.summary
    }
}
struct WebServerSummary;

async fn websocket_handler(
    ws: axum::extract::WebSocketUpgrade,
    axum::extract::State(stage): axum::extract::State<ActorHandle<DownstreamClientMessage>>,
) -> axum::response::Response {
    ws.on_upgrade(async |socket| {
        let mut socket: axum::extract::ws::WebSocket = socket;
        let stage: ActorHandle<DownstreamClientMessage> = stage;

        'recv_messages: while let Some(Ok(message)) =
            futures_util::StreamExt::next(&mut socket).await
        {
            let msg_raw: axum::extract::ws::Message = message;
            let msg_valid: DownstreamClientMessage = match (&msg_raw).try_into() {
                Ok(n) => n,
                Err(err) => {
                    log::error!("invalid message from downstream client: {err:?}: {msg_raw:?}");
                    continue 'recv_messages;
                }
            };
            if let Err(err) = stage.send(msg_valid).await {
                log::error!("could not send message from downstream client to stage: {err:?}");
                continue 'recv_messages;
            };
        }
    })
}

#[derive(Clone, Debug, serde::Deserialize)]
enum DownstreamClientMessage {
    ServerSaveAndClose,
    ServerConfigure { cfg: GameServerConfigurationPatch },
    ServerInstallOrUpdateAndStart,
    GameWorldKillPlayer { id: String },
}
#[derive(Clone, Debug, serde::Deserialize)]
struct GameServerConfigurationPatch {}

impl TryFrom<&axum::extract::ws::Message> for DownstreamClientMessage {
    type Error = ();

    fn try_from(value: &axum::extract::ws::Message) -> Result<Self, Self::Error> {
        let utf8: String = match value {
            axum::extract::ws::Message::Text(utf8_bytes) => utf8_bytes.to_string(),
            axum::extract::ws::Message::Binary(_)
            | axum::extract::ws::Message::Ping(_)
            | axum::extract::ws::Message::Pong(_)
            | axum::extract::ws::Message::Close(_) => return Err(()),
        };
        let message: DownstreamClientMessage = match serde_json::from_str(&utf8) {
            Ok(n) => n,
            Err(_) => return Err(()),
        };
        return Ok(message);
    }
}

/// A thing that _works_ in a coroutine (alias _background task_), working with
/// incoming _messages_ that are sent from somewhere via the exposed _handle_.
trait Actor<Message> {
    type Summary;

    fn new(terminator: &Terminator) -> Self;

    /// Get a handle for sending messages to the actor.
    fn get_handle(&self) -> ActorHandle<Message>;

    async fn work(self) -> Self::Summary;
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
        return ActorHandle::new(tx.clone());
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
                        "failed to send downstream client message from stage to game server controller: {err}"
                    );
                }
            }
        });
        _ = self.cancel_read.run_until_cancelled(coroutine).await;
        return self.summary;
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
    should_mock: bool,
}
impl Store {
    pub fn new(cli_args: &CliArgs) -> std::sync::Arc<tokio::sync::Mutex<Self>> {
        std::sync::Arc::new(tokio::sync::Mutex::new(Self {
            should_mock: cli_args.mock,
        }))
    }

    pub async fn get_config(&self) -> Configuration {
        let game_world_size = 1000;
        let game_world_seed = 1337;

        if self.should_mock {
            let mut root_dir_absolute = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            root_dir_absolute.push("../mocks");
            root_dir_absolute = root_dir_absolute.canonicalize().unwrap();

            return Configuration {
                root_dir_absolute,
                installer_relative: std::path::Path::new("steamcmd/target/debug/steamcmd")
                    .to_path_buf(),
                game_relative: std::path::Path::new("RustDedicated/target/debug/RustDedicated")
                    .to_path_buf(),
                manifest_relative: std::path::Path::new("dummy_manifest.acf").to_path_buf(),

                game_world_size,
                game_world_seed,
            };
        } else {
            todo!()
        }
    }
}

fn extract_buildid_from_buf(buf: &str) -> Option<u32> {
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
                return None;
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
