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

    let store: std::sync::Arc<tokio::sync::Mutex<Store>> = Store::new();

    let game_ctl: GameServerController = GameServerController::new(&terminator);

    /*
     * TODO: Define actors for "game server controller" and "RCON client", and
     *       give handle (i.e. `Actor::get_handle()`) of each to `stage`, so
     *       that `stage` can send downstream client messages to each actor
     *       depending on the message variant.
     */

    /*
     * Stage on which downstream WebSocket clients communicate.
     */
    let stage = Stage::new(&terminator); // "actors", hence "stage" :D

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
    cancel: tokio_util::sync::CancellationToken,
}
impl GameServerController {
    fn new(terminator: &Terminator) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel::<DownstreamClientMessage>(1);
        Self {
            summary: GameServerControllerSummary,
            cancel: terminator.get_handle(),
            tx,
            rx,
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
            GameServerStateMachine::init()
                .loop_transitions(self.rx, store)
                .await;
        });
        _ = self.cancel.run_until_cancelled(coroutine).await;
        return self.summary;
    }
}
struct GameServerControllerSummary;

#[derive(Debug)]
enum GameServerStateMachine {
    Init,
    Preparing,
    InstalledAndConfigured { cfg: LaunchConfiguration },
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
    ) -> ! {
        loop {
            match self {
                Self::Init => {
                    /*
                     * TODO: Transition automatically to "Preparing".
                     */
                    self = Self::Preparing;
                }

                Self::Preparing => {
                    /*
                     * TODO: Install or update `RustDedicated` using `steamcmd`.
                     */

                    let cfg: LaunchConfiguration;
                    {
                        let lock = store.lock().await;
                        cfg = lock.get_game_server_launch_configuration().await;
                    }

                    self = Self::InstalledAndConfigured { cfg };
                }

                Self::InstalledAndConfigured { cfg } => {
                    let cfg: LaunchConfiguration = cfg;
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
                                    "ignoring unexpected command: {command:?} for current state: {self:?}"
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
                    /*
                     * TODO: Wait for command to "update and restart". Then
                     *       transition to "Preparing".
                     */
                    let msg = command_rx.recv().await;
                    if let Some(command) = msg {
                        let command: DownstreamClientMessage = command;
                        match command {
                            DownstreamClientMessage::ServerInstallOrUpdateAndStart => {
                                self = Self::Preparing;
                            }
                            _ => {
                                log::error!(
                                    "ignoring unexpected command: {command:?} for current state: {self:?}"
                                );
                            }
                        }
                    }
                }

                Self::TerminatedUnexpectedly => {
                    /*
                     * TODO: Transition automatically to "Preparing".
                     */
                }
            }
        }
    }
}

#[derive(Debug)]
struct LaunchConfiguration {}

struct Terminator {
    summary: TerminatorSummary,
    cancel: tokio_util::sync::CancellationToken,
}
impl Terminator {
    pub fn new() -> Self {
        Self {
            cancel: tokio_util::sync::CancellationToken::new(),
            summary: TerminatorSummary {},
        }
    }

    pub async fn work(self) -> TerminatorSummary {
        let coroutine = tokio::spawn(async move {
            let mut sigint =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
            tokio::select! {
                _ = sigint.recv() => log::info!("SIGINT"),
                _ = sigterm.recv() => log::info!("SIGTERM"),
            }
            self.cancel.cancel();
        });
        _ = coroutine.await;
        self.summary
    }

    pub fn get_handle(&self) -> tokio_util::sync::CancellationToken {
        return self.cancel.child_token();
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
    cancel: tokio_util::sync::CancellationToken,
    router: axum::Router,
}
impl WebServer {
    pub fn new(terminator: &Terminator, stage: &Stage) -> Self {
        let router: axum::Router = axum::Router::new()
            .route("/ws", axum::routing::get(websocket_handler))
            .with_state(stage.get_handle());

        Self {
            summary: WebServerSummary {},
            cancel: terminator.get_handle(),
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
            .cancel
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
    cancel: tokio_util::sync::CancellationToken,
    channel: (
        tokio::sync::mpsc::Sender<DownstreamClientMessage>,
        tokio::sync::mpsc::Receiver<DownstreamClientMessage>,
    ),
}
impl Actor<DownstreamClientMessage> for Stage {
    type Summary = StageSummary;

    fn new(terminator: &Terminator) -> Self {
        Self {
            channel: tokio::sync::mpsc::channel(1),
            cancel: terminator.get_handle(),
            summary: StageSummary { messages_total: 0 },
        }
    }

    fn get_handle(&self) -> ActorHandle<DownstreamClientMessage> {
        let (tx, _rx) = &self.channel;
        return ActorHandle::new(tx.clone());
    }

    async fn work(mut self) -> Self::Summary {
        let (_tx, mut rx) = self.channel;
        let coroutine = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Some(no_overflow) = self.summary.messages_total.checked_add(1) {
                    self.summary.messages_total = no_overflow;
                }
                let msg: DownstreamClientMessage = msg;
                match msg {
                    /*
                     * TODO: Send commands to yet another actor(s) that control
                     *       the game server and/or the RCON socket.
                     */
                    DownstreamClientMessage::ServerSaveAndClose => {
                        println!("NOTE: ServerSaveAndClose")
                    }
                    DownstreamClientMessage::ServerConfigure { cfg } => {
                        println!("NOTE: ServerConfigure: {cfg:?}")
                    }
                    DownstreamClientMessage::ServerInstallOrUpdateAndStart => {
                        println!("NOTE: ServerInstallOrUpdateAndStart")
                    }
                    DownstreamClientMessage::GameWorldKillPlayer { id } => {
                        println!("NOTE: GameWorldKillPlayer: {id:?}")
                    }
                }
            }
        });
        _ = self.cancel.run_until_cancelled(coroutine).await;
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

struct Store {}
impl Store {
    pub fn new() -> std::sync::Arc<tokio::sync::Mutex<Self>> {
        std::sync::Arc::new(tokio::sync::Mutex::new(Self {}))
    }

    pub async fn get_game_server_launch_configuration(&self) -> LaunchConfiguration {
        LaunchConfiguration {}
    }
}
