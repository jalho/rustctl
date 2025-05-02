fn main() {
    let shared: std::sync::Arc<tokio::sync::Mutex<SharedState>> = SharedState::init();

    let app: axum::Router = axum::Router::new()
        .route("/", axum::routing::get(webpage))
        .route(
            ROUTE_CONFIG.route_path_sock,
            axum::routing::get(axum::routing::get(handle_websocket_upgrade)),
        )
        .fallback(axum::routing::get(no_content))
        .with_state(shared);

    let runtime: tokio::runtime::Runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        let listener: tokio::net::TcpListener = tokio::net::TcpListener::bind("127.0.0.1:8080")
            .await
            .unwrap();

        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });
}

#[derive(Debug)]
struct RouteConfig {
    route_path_sock: &'static str,
}

const ROUTE_CONFIG: RouteConfig = RouteConfig {
    route_path_sock: "/sock",
};

async fn no_content() -> axum::http::StatusCode {
    axum::http::StatusCode::NO_CONTENT
}

async fn webpage(
    shared: axum::extract::State<std::sync::Arc<tokio::sync::Mutex<SharedState>>>,
) -> axum::response::Html<String> {
    let clients_count: usize;
    {
        let shared_locked: tokio::sync::MutexGuard<SharedState> = shared.lock().await;
        clients_count = shared_locked.clients.len();
    }

    let content: String = format!(
        r#"<!DOCTYPE html>
<html>
<body>
    <p>{clients_count} clients connected</p>
    <button onclick="ws.send('foobar')">Send 'foobar'</button>
</body>
<script>
    const ws = new WebSocket("{path_sock}");
    ws.addEventListener("message", (message) => {{
        console.log(message.data);
    }});
</script>
</html>"#,
        path_sock = ROUTE_CONFIG.route_path_sock
    );

    axum::response::Html(content)
}

async fn handle_websocket_upgrade(
    shared: axum::extract::State<std::sync::Arc<tokio::sync::Mutex<SharedState>>>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    ws: axum::extract::WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    {
        let mut shared_locked: tokio::sync::MutexGuard<SharedState> = shared.lock().await;
        shared_locked.clients.insert(addr, Client::new());
    }
    ws.on_upgrade(move |sock| send_and_receive_messages(shared, addr, sock))
}

#[derive(Debug)]
struct Message {
    timestamp: u64,
    content: String,
}

impl Message {
    pub fn new(timestamp: u64, content: String) -> Self {
        Self { timestamp, content }
    }
}

#[derive(Debug)]
struct Client {
    messages: std::collections::VecDeque<Message>,
}

impl Client {
    pub fn new() -> Self {
        Self {
            messages: std::collections::VecDeque::new(),
        }
    }
}

#[derive(Debug)]
struct SharedState {
    timestamp: Option<u64>,
    clients: std::collections::HashMap<std::net::SocketAddr, Client>,
}

impl SharedState {
    pub fn init() -> std::sync::Arc<tokio::sync::Mutex<Self>> {
        std::sync::Arc::new(tokio::sync::Mutex::new(Self {
            timestamp: None,
            clients: std::collections::HashMap::new(),
        }))
    }

    pub fn serialize(&self) -> axum::extract::ws::Message {
        axum::extract::ws::Message::Text(axum::extract::ws::Utf8Bytes::from(format!("{self:?}")))
    }
}

const MAX_MESSAGES_PER_CLIENT: usize = 16;

async fn send_and_receive_messages(
    shared: axum::extract::State<std::sync::Arc<tokio::sync::Mutex<SharedState>>>,
    addr: std::net::SocketAddr,
    sock: axum::extract::ws::WebSocket,
) {
    let (mut sock_tx, mut sock_rx): (
        futures::stream::SplitSink<axum::extract::ws::WebSocket, axum::extract::ws::Message>,
        futures::stream::SplitStream<axum::extract::ws::WebSocket>,
    ) = futures::StreamExt::split(sock);

    let shared_rx: std::sync::Arc<tokio::sync::Mutex<SharedState>> = std::sync::Arc::clone(&shared);
    let rx = tokio::spawn(async move {
        loop {
            let recv: Option<Result<axum::extract::ws::Message, axum::Error>> =
                futures::StreamExt::next(&mut sock_rx).await;

            match recv {
                Some(Ok(axum::extract::ws::Message::Text(msg))) => {
                    let mut shared = shared_rx.lock().await;
                    match shared.clients.get_mut(&addr) {
                        Some(initalized) => {
                            if initalized.messages.len() >= MAX_MESSAGES_PER_CLIENT {
                                initalized.messages.pop_front();
                            }
                            initalized
                                .messages
                                .push_back(Message::new(now(), msg.to_string()));
                        }
                        None => {
                            let mut client = Client::new();
                            client
                                .messages
                                .push_front(Message::new(now(), msg.to_string()));
                            shared.clients.insert(addr, client);
                        }
                    }
                }
                _ => {
                    break;
                }
            }
        }
    });

    let shared_tx: std::sync::Arc<tokio::sync::Mutex<SharedState>> = std::sync::Arc::clone(&shared);
    let tx = tokio::spawn(async move {
        let mut interval: tokio::time::Interval =
            tokio::time::interval(std::time::Duration::from_millis(300));
        loop {
            interval.tick().await;

            {
                let mut shared_locked: tokio::sync::MutexGuard<SharedState> =
                    shared_tx.lock().await;
                shared_locked.timestamp = Some(now());

                let send_result: Result<(), axum::Error> =
                    futures::SinkExt::send(&mut sock_tx, shared_locked.serialize()).await;
                if send_result.is_err() {
                    break;
                }
            }
        }
    });

    _ = rx.await;
    _ = tx.await;

    {
        let mut shared_locked: tokio::sync::MutexGuard<SharedState> = shared.lock().await;
        shared_locked.clients.remove(&addr);
    }
}

fn now() -> u64 {
    let now: std::time::SystemTime = std::time::SystemTime::now();
    let duration_since_epoch: std::time::Duration =
        now.duration_since(std::time::UNIX_EPOCH).unwrap();
    let timestamp: u64 = duration_since_epoch.as_secs();
    timestamp
}
