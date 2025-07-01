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
    let router = axum::Router::new()
        .route("/ws", axum::routing::get(websocket_handler))
        .with_state(stage.get_handle());

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .enable_all()
        .build()
    {
        Ok(n) => n,
        Err(err) => {
            eprintln!("failed to build async runtime: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let runtime_done = runtime.block_on(async {
        let tcp_listener = match tokio::net::TcpListener::bind("127.0.0.1:8080").await {
            Ok(n) => n,
            Err(err) => {
                eprintln!("failed to bind TCP listener: {err}");
                return Err(err);
            }
        };

        /*
         * TODO: Make the web server an actor with cancel token?
         */
        _ = tokio::join!(axum::serve(tcp_listener, router), stage.work());

        return Ok(());
    });

    match runtime_done {
        Ok(_) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("something went wrong in async runtime: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

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
                    eprintln!("invalid message from downstream client: {err:?}: {msg_raw:?}");
                    continue 'recv_messages;
                }
            };
            if let Err(err) = stage.send(msg_valid).await {
                eprintln!("could not send message from downstream client to stage: {err:?}");
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

trait Actor<Message> {
    fn new(cancel: tokio_util::sync::CancellationToken) -> Self;
    fn get_handle(&self) -> ActorHandle<Message>;
}

struct Stage {
    cancel: tokio_util::sync::CancellationToken,
    channel: (
        tokio::sync::mpsc::Sender<DownstreamClientMessage>,
        tokio::sync::mpsc::Receiver<DownstreamClientMessage>,
    ),
}
impl Stage {
    pub async fn work(self) {
        let (_tx, mut rx) = self.channel;
        let coroutine = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
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
            eprintln!("coroutine failed: {err}");
        }
    }
}
impl Actor<DownstreamClientMessage> for Stage {
    fn new(cancel: tokio_util::sync::CancellationToken) -> Self {
        Self {
            channel: tokio::sync::mpsc::channel(1),
            cancel,
        }
    }

    fn get_handle(&self) -> ActorHandle<DownstreamClientMessage> {
        let (tx, _rx) = &self.channel;
        return ActorHandle::new(tx.clone());
    }
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
