use core::SharedState;
use std::sync::Arc;
use tokio::{runtime, sync::Mutex};

mod game {
    use crate::{constants::INTERVAL_FETCH_GAME_STATE, core::SharedState};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    pub async fn read_state(shared: Arc<Mutex<SharedState>>) {
        let mut interval = tokio::time::interval(INTERVAL_FETCH_GAME_STATE);
        loop {
            interval.tick().await;

            let state = GameState::read();

            {
                let mut shared = shared.lock().await;
                shared.game = state;
            }
        }
    }

    /// State of the game (obtained via RCON).
    #[derive(serde::Serialize)]
    pub struct GameState {
        /// Time of day in the game world.
        time_of_day: f64,

        players: std::collections::HashMap<Identifier, Player>,

        toolcupboards: std::collections::HashMap<Identifier, Toolcupboard>,
    }

    impl GameState {
        pub fn read() -> Self {
            // TODO: Query game state via RCON
            Self {
                time_of_day: 0.0,
                players: std::collections::HashMap::new(),
                toolcupboards: std::collections::HashMap::new(),
            }
        }
    }

    #[derive(serde::Serialize)]
    struct Identifier;

    #[derive(serde::Serialize)]
    struct Location;

    #[derive(serde::Serialize)]
    struct Toolcupboard {
        id: Identifier,
        location: Location,
    }

    #[derive(serde::Serialize)]
    struct Player {
        id: Identifier,
        location: Location,
    }
}

mod constants {
    use std::time::Duration;

    pub const ADDR_WEB_SERVICE_LISTEN: &str = "0.0.0.0:8080";

    pub const INTERVAL_MONITOR_SYSTEM: Duration = Duration::from_millis(500);

    pub const INTERVAL_FETCH_GAME_STATE: Duration = Duration::from_millis(200);

    pub const INTERVAL_SYNC_CLIENT: Duration = Duration::from_millis(200);

    pub const URL_PATH_WEBSOCKET_CONNECT: &str = "/sock";

    pub const MESSAGES_PER_CLIENT_INMEM_MAX: usize = 16;
}

mod core {
    use crate::{
        constants::{INTERVAL_SYNC_CLIENT, MESSAGES_PER_CLIENT_INMEM_MAX},
        game::GameState,
        system::SystemState,
    };
    use axum::{
        extract::{
            ConnectInfo, State, WebSocketUpgrade,
            ws::{Message, Utf8Bytes, WebSocket},
        },
        response::IntoResponse,
    };
    use futures::{SinkExt, StreamExt};
    use std::{
        collections::{HashMap, VecDeque},
        net::SocketAddr,
        sync::Arc,
        time::SystemTime,
    };
    use tokio::sync::{Mutex, MutexGuard};

    pub async fn handle_websocket_upgrade(
        shared: State<Arc<Mutex<SharedState>>>,
        ConnectInfo(addr): ConnectInfo<SocketAddr>,
        ws: WebSocketUpgrade,
    ) -> impl IntoResponse {
        {
            let mut shared_locked: MutexGuard<SharedState> = shared.lock().await;
            shared_locked.clients.insert(addr, Client::new());
        }
        ws.on_upgrade(move |sock| send_and_receive_messages(shared, addr, sock))
    }

    async fn send_and_receive_messages(
        shared: State<Arc<Mutex<SharedState>>>,
        addr: SocketAddr,
        sock: WebSocket,
    ) {
        let (mut sock_tx, mut sock_rx) = StreamExt::split(sock);

        let shared_rx: Arc<Mutex<SharedState>> = Arc::clone(&shared);
        let rx = tokio::spawn(async move {
            loop {
                let recv = StreamExt::next(&mut sock_rx).await;

                match recv {
                    Some(Ok(Message::Text(msg))) => {
                        let mut shared = shared_rx.lock().await;
                        match shared.clients.get_mut(&addr) {
                            Some(initalized) => {
                                if initalized.messages.len() >= MESSAGES_PER_CLIENT_INMEM_MAX {
                                    initalized.messages.pop_front();
                                }
                                initalized
                                    .messages
                                    .push_back(Command::new(now(), msg.to_string()));
                            }
                            None => {
                                let mut client = Client::new();
                                client
                                    .messages
                                    .push_front(Command::new(now(), msg.to_string()));
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

        let shared_tx: Arc<Mutex<SharedState>> = Arc::clone(&shared);
        let tx = tokio::spawn(async move {
            let mut interval = tokio::time::interval(INTERVAL_SYNC_CLIENT);
            loop {
                interval.tick().await;

                {
                    let shared_locked: MutexGuard<SharedState> = shared_tx.lock().await;
                    let sent = SinkExt::send(&mut sock_tx, shared_locked.serialize()).await;
                    if sent.is_err() {
                        break;
                    }
                }
            }
        });

        _ = rx.await;
        _ = tx.await;

        {
            let mut shared = shared.lock().await;
            shared.clients.remove(&addr);
        }
    }

    #[derive(serde::Serialize)]
    pub struct SharedState {
        pub clients: HashMap<SocketAddr, Client>,
        pub game: GameState,
        pub system: SystemState,
    }

    impl SharedState {
        pub fn init() -> Arc<Mutex<Self>> {
            Arc::new(Mutex::new(Self {
                clients: HashMap::new(),
                game: GameState::read(),
                system: SystemState::read(),
            }))
        }

        pub fn serialize(&self) -> Message {
            let json: String = serde_json::to_string(&self).unwrap();
            Message::Text(Utf8Bytes::from(json))
        }
    }

    #[derive(serde::Serialize)]
    struct Command {
        timestamp: u64,
        content: String,
    }

    impl Command {
        pub fn new(timestamp: u64, content: String) -> Self {
            Self { timestamp, content }
        }
    }

    #[derive(serde::Serialize)]
    pub struct Client {
        messages: VecDeque<Command>,
    }

    impl Client {
        pub fn new() -> Self {
            Self {
                messages: VecDeque::new(),
            }
        }
    }

    fn now() -> u64 {
        let now = SystemTime::now();
        let dur: std::time::Duration = now.duration_since(std::time::UNIX_EPOCH).unwrap();
        let timestamp: u64 = dur.as_secs();
        timestamp
    }
}

mod system {
    use crate::{constants::INTERVAL_MONITOR_SYSTEM, core::SharedState};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(serde::Serialize)]
    struct ResourceUsage {
        process_id: (),
        cpu: (),
        memory: (),
    }

    #[derive(serde::Serialize)]
    pub struct SystemState {
        game_server: Option<ResourceUsage>,
        game_server_installer: Option<ResourceUsage>,
    }

    impl SystemState {
        pub fn read() -> Self {
            Self {
                game_server: None,
                game_server_installer: None,
            }
        }
    }

    pub async fn monitor_usage(shared: Arc<Mutex<SharedState>>) {
        let mut interval = tokio::time::interval(INTERVAL_MONITOR_SYSTEM);
        loop {
            interval.tick().await;

            // TODO: Read system CPU, network, memory usage etc.

            let usage = SystemState::read();

            {
                let mut shared = shared.lock().await;
                shared.system = usage;
            }
        }
    }
}

mod web {
    use crate::{
        constants::{ADDR_WEB_SERVICE_LISTEN, URL_PATH_WEBSOCKET_CONNECT},
        core::{SharedState, handle_websocket_upgrade},
    };
    use axum::{Router, http::StatusCode, response::Html, routing};
    use std::{net::SocketAddr, sync::Arc};
    use tokio::sync::Mutex;

    pub async fn start(shared: Arc<Mutex<SharedState>>) {
        let web_service = Router::new()
            .route("/", routing::get(content_main))
            .route(
                URL_PATH_WEBSOCKET_CONNECT,
                routing::get(routing::get(handle_websocket_upgrade)),
            )
            .fallback(routing::get(no_content))
            .with_state(shared);

        let listener = tokio::net::TcpListener::bind(ADDR_WEB_SERVICE_LISTEN)
            .await
            .unwrap();

        axum::serve(
            listener,
            web_service.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    }

    async fn no_content() -> StatusCode {
        StatusCode::NO_CONTENT
    }

    async fn content_main() -> Html<String> {
        let content: String = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <style>
        html {{
            background-color: #121212;
            color: #e0e0e0;
            font-family: sans-serif;
        }}
        body {{
            margin: 2em;
        }}
        button {{
            background-color: #1e1e1e;
            color: #ffffff;
            border: 1px solid #333;
            padding: 0.5em 1em;
            cursor: pointer;
        }}
        pre {{
            background-color: #1e1e1e;
            color: #c0c0c0;
            padding: 1em;
            border-radius: 0.5em;
            overflow-x: auto;
        }}
    </style>
</head>
<body>
    <button onclick="ws.send('foobar')">Send 'foobar'</button>
    <pre><code id="output"></code></pre>
</body>
<script>
    const ws = new WebSocket("{path_sock}");
    const output = document.getElementById("output");
    ws.addEventListener("message", (message) => {{
        const text = JSON.stringify(JSON.parse(message.data), null, 2);
        output.textContent = text;
    }});
</script>
</html>"#,
            path_sock = URL_PATH_WEBSOCKET_CONNECT
        );

        Html(content)
    }
}

fn main() {
    let state: Arc<Mutex<SharedState>> = SharedState::init();

    let runtime = runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        tokio::spawn(system::monitor_usage(state.clone()));
        tokio::spawn(game::read_state(state.clone()));
        web::start(state).await;
    });
}
