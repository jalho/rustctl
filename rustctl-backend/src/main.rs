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
    let args: Args = <Args as clap::Parser>::parse();

    let _handle: log4rs::Handle = init_logging(args.log_level);
    log::info!(
        "{name} {version}",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    );

    let cancel = tokio_util::sync::CancellationToken::new();

    /*
     * TODO: Define actors for "game server controller" and "RCON client", and
     *       give handle (i.e. `Actor::get_handle()`) of each to `stage`, so
     *       that `stage` can send downstream client messages to each actor
     *       depending on the message variant.
     */

    /*
     * Stage (an actor) on which downstream WebSocket clients communicate (who
     * are also actors).
     */
    let stage = Stage::new(cancel.child_token());

    /*
     * Kind of an actor as well, but the messages are coming from underlying
     * abstractions. Sends messages to the _stage_ actor.
     */
    let web_server = WebServer::new(cancel.child_token(), &stage);

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

    let _runtime_done: (WebServerSummary, StageSummary) = runtime.block_on(async {
        let summary = tokio::join!(web_server.work(), stage.work());
        return summary;
    });

    std::process::ExitCode::SUCCESS
}

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
pub struct Args {
    #[arg(short, long, default_value_t = log::LevelFilter::Debug)]
    pub log_level: log::LevelFilter,
}

struct WebServer {
    summary: WebServerSummary,
    cancel: tokio_util::sync::CancellationToken,
    router: axum::Router,
}
impl WebServer {
    pub fn new(cancel: tokio_util::sync::CancellationToken, stage: &Stage) -> Self {
        let router: axum::Router = axum::Router::new()
            .route("/ws", axum::routing::get(websocket_handler))
            .with_state(stage.get_handle());

        Self {
            summary: WebServerSummary {},
            cancel,
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

    fn new(cancel: tokio_util::sync::CancellationToken) -> Self;

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

    fn new(cancel: tokio_util::sync::CancellationToken) -> Self {
        Self {
            channel: tokio::sync::mpsc::channel(1),
            cancel,
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
        let coroutine_done = self.cancel.run_until_cancelled(coroutine).await;
        if let Some(Err(err)) = coroutine_done {
            log::error!("coroutine failed: {err}");
        }
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
