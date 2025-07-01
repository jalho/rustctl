/*
 * TODO: Rewrite in terms of the "actor pattern" (a concurrency pattern): There
 *       should be _actors_ that own their stuff (such as I/O resources), and
 *       that perform work in coroutines (alias "background tasks"), and that
 *       may communicate with other actors via various _channels_. The main
 *       components of the program should all be actors, and the program's
 *       main functionality should be implemented by arranging channels between
 *       actors.
 */
fn main() {
    let stage = Actor::<Stage, DownstreamClientMessage>::new(Stage::new());
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
            let tcp_listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
                .await
                .unwrap();
            axum::serve(tcp_listener, router).await.unwrap();
        });
        _ = tokio::join!(coroutine_web);
    });
}

async fn websocket_handler(
    ws: axum::extract::WebSocketUpgrade,
    axum::extract::State(stage): axum::extract::State<
        tokio::sync::mpsc::Sender<DownstreamClientMessage>,
    >,
) -> axum::response::Response {
    ws.on_upgrade(async |socket| {
        let _socket: axum::extract::ws::WebSocket = socket;
        let _stage = stage;
    })
}

struct DownstreamClientMessage;

struct Actor<Resource, Message> {
    managed_resource: Resource,
    channel: (
        tokio::sync::mpsc::Sender<Message>,
        tokio::sync::mpsc::Receiver<Message>,
    ),
}
impl<Resource, Message> Actor<Resource, Message> {
    pub fn new(managed_resource: Resource) -> Self {
        Self {
            managed_resource,
            channel: tokio::sync::mpsc::channel(1),
        }
    }

    pub fn get_handle(&self) -> &tokio::sync::mpsc::Sender<Message> {
        let (tx, _rx) = &self.channel;
        return tx;
    }
}

/// _Stage_ is a thing on which _actors_ communicate :D Stage itself is also an
/// actor, in the context of the concurrency pattern where _actors_ are a thing.
struct Stage {}
impl Stage {
    pub fn new() -> Self {
        Self {}
    }
}
