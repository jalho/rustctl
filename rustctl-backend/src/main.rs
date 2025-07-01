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
fn main() {
    // TODO: Should cancel.child_token() be given to Stage::new()?
    let cancel = tokio_util::sync::CancellationToken::new();

    /*
     * Stage on which downstream WebSocket client actors communicate.
     */
    let stage = Actor::<DownstreamClientMessage>::new();
    let router = axum::Router::new()
        .route("/ws", axum::routing::get(websocket_handler))
        .with_state(stage.get_handle());

    let runtime = tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        let coroutine_web = tokio::spawn(async move {
            let tcp_listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
                .await
                .unwrap();
            axum::serve(tcp_listener, router).await.unwrap();
        });

        let coroutine_stage = tokio::spawn(stage.work(cancel.child_token()));

        _ = tokio::join!(coroutine_web, coroutine_stage);
    });
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
    A(GameServerControl),
    B(GameWorldControl),
}
#[derive(Clone, Debug, serde::Deserialize)]
enum GameServerControl {
    SaveAndClose,
    Configure { configuration: ConfigurationPayload },
    InstallOrUpdateAndStart,
}
#[derive(Clone, Debug, serde::Deserialize)]
enum GameWorldControl {
    KillPlayer,
}
#[derive(Clone, Debug, serde::Deserialize)]
struct ConfigurationPayload {}

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

trait ReceiverHolder<Message> {
    fn get_receiver(self) -> tokio::sync::mpsc::Receiver<Message>;
}

struct Actor<Message> {
    channel: (
        tokio::sync::mpsc::Sender<Message>,
        tokio::sync::mpsc::Receiver<Message>,
    ),
}
impl<Message> Actor<Message>
where
    Message: std::fmt::Debug,
{
    pub fn new() -> Self {
        Self {
            channel: tokio::sync::mpsc::channel(1),
        }
    }

    pub fn get_handle(&self) -> ActorHandle<Message> {
        let (tx, _rx) = &self.channel;
        return ActorHandle::new(tx.clone());
    }

    pub async fn work(self, cancel: tokio_util::sync::CancellationToken) {
        let (tx, rx) = self.channel;
        let coroutine = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let msg: Message = msg;
                println!("TODO: Actor::work working on message: {msg:?}");
            }
        });
        let coroutine_done = cancel.run_until_cancelled(coroutine).await;
        if let Some(Err(err)) = coroutine_done {
            eprintln!("coroutine failed: {err}");
        }
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
