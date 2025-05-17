use crate::{
    constants::{COOKIE_NAME_SESSION, INTERVAL_SYNC_CLIENT},
    game::GameState,
    system::SystemState,
    web::{AppState, ClientSession},
};
use axum::{
    extract::{
        ConnectInfo, State, WebSocketUpgrade,
        ws::{Message, Utf8Bytes, WebSocket},
    },
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::SignedCookieJar;
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;

pub async fn handle_websocket_upgrade(
    ws: WebSocketUpgrade,
    jar: SignedCookieJar,
    state: State<AppState>,
    connect_info: ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    match jar
        .get(COOKIE_NAME_SESSION)
        .and_then(|cookie| serde_json::from_str::<ClientSession>(cookie.value()).ok())
    {
        Some(_) => {
            let shared_state = Arc::clone(&state.shared);
            ws.on_upgrade(async move |sock| {
                let client = Client::new(connect_info.0, sock, Arc::clone(&shared_state));

                let client_id: Uuid;
                {
                    let mut lock = shared_state.lock().await;
                    client_id = lock.register(&client);
                }

                client.send_and_receive_messages().await;

                {
                    let mut lock = shared_state.lock().await;
                    lock.unregister(&client_id);
                }
            })
        }
        None => StatusCode::UNAUTHORIZED.into_response(),
    }
}

#[derive(Clone)]
enum ClientIdentity {
    Anonymous,
    // could add e.g. variant SteamUser here (encapsulating Steam ID)
}

impl serde::Serialize for ClientIdentity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s: &str = match self {
            ClientIdentity::Anonymous => "Anonymous",
        };
        return serializer.serialize_str(s);
    }
}

#[derive(serde::Serialize, Clone)]
struct ClientMeta {
    #[serde(skip_serializing)]
    _addr: std::net::SocketAddr,

    #[serde(serialize_with = "serialize_date_utc_js")]
    connected_at: chrono::DateTime<chrono::Utc>,

    identity: ClientIdentity,
}

/// Serialize into JavaScript compatible date notation, e.g. "2025-01-01T00:00:00.000Z"
fn serialize_date_utc_js<S>(
    dt: &chrono::DateTime<chrono::Utc>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let timestamp: String = format!(
        "{}.{:03}Z",
        dt.format("%Y-%m-%dT%H:%M:%S"),
        dt.timestamp_subsec_millis()
    );
    return serializer.serialize_str(&timestamp);
}

/// State update payload to be sent to clients regularly.
/// (This type exists in the frontend code too, using the same name.)
#[derive(serde::Serialize)]
struct TWebSocketStateUpdatePayload {
    clients: HashMap<Uuid, ClientMeta>,
    game: GameState,
    system: SystemState,
}

impl From<&SharedState> for TWebSocketStateUpdatePayload {
    fn from(value: &SharedState) -> Self {
        Self {
            clients: value.clients.clone(),
            game: value.game.clone(),
            system: value.system.clone(),
        }
    }
}

#[derive(Clone)]
pub struct SharedState {
    clients: HashMap<Uuid, ClientMeta>,
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

    fn register(&mut self, client: &Client) -> Uuid {
        let client_id = Uuid::new_v4();
        let client_meta = ClientMeta {
            _addr: client.addr,
            connected_at: Utc::now(),
            identity: ClientIdentity::Anonymous,
        };

        self.clients.insert(client_id, client_meta);

        client_id
    }

    fn unregister(&mut self, client_id: &Uuid) {
        self.clients.remove(client_id);
    }
}

impl From<&SharedState> for Message {
    fn from(value: &SharedState) -> Self {
        let payload: TWebSocketStateUpdatePayload = value.into();
        let json: String = serde_json::to_string(&payload).unwrap();
        Message::Text(Utf8Bytes::from(json))
    }
}

#[derive(serde::Serialize, serde::Deserialize, Hash, Eq, PartialEq, Debug, Clone)]
#[serde(tag = "_type", content = "payload")]
pub enum Command {
    InstallOrUpdateAndStart,
    Stop,
}

impl Command {
    pub fn new(serialized: &str) -> Self {
        serde_json::from_str(serialized).unwrap()
    }
}

pub struct Client {
    addr: SocketAddr,
    sock: WebSocket,
    shared: Arc<Mutex<SharedState>>,
}

impl Client {
    pub fn new(addr: SocketAddr, sock: WebSocket, shared: Arc<Mutex<SharedState>>) -> Self {
        Self { addr, sock, shared }
    }

    async fn send_and_receive_messages(self) {
        let (mut sock_tx, mut sock_rx) = StreamExt::split(self.sock);

        let shared_rx: Arc<Mutex<SharedState>> = Arc::clone(&self.shared);
        let mut task_rx_cmd = tokio::task::Builder::new()
            .name("recv_commands")
            .spawn(async move {
                loop {
                    let recv = StreamExt::next(&mut sock_rx).await;

                    match recv {
                        Some(Ok(Message::Text(msg))) => {
                            let command = Command::new(&msg.to_string());

                            // TODO: Do a state transition based on the received command
                            println!("TODO: Transition state: {command:?}");
                        }
                        _ => {
                            break;
                        }
                    }
                }
            })
            .unwrap();

        let shared_tx: Arc<Mutex<SharedState>> = Arc::clone(&self.shared);
        let mut task_tx_state = tokio::task::Builder::new()
            .name("send_state")
            .spawn(async move {
                let mut interval = tokio::time::interval(INTERVAL_SYNC_CLIENT);
                loop {
                    interval.tick().await;

                    let snapshot: SharedState;
                    {
                        let shared_locked: MutexGuard<SharedState> = shared_tx.lock().await;
                        snapshot = shared_locked.clone();
                    }

                    let serialized: Message = (&snapshot).into();
                    let sent = SinkExt::send(&mut sock_tx, serialized).await;
                    if sent.is_err() {
                        break;
                    }
                }
            })
            .unwrap();

        tokio::select! {
            _ = (&mut task_rx_cmd) => {
                task_tx_state.abort();
            },
            _ = (&mut task_tx_state) => {
                task_rx_cmd.abort();
            }
        }
    }
}

#[derive(clap::Parser)]
#[command(name = "rustctl")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: CliCommand,
}

impl Cli {
    pub fn get_args() -> Self {
        <Cli as clap::Parser>::parse()
    }
}

#[derive(clap::Subcommand)]
pub enum CliCommand {
    Start {
        #[arg(long = "web-root", short, value_name = "PATH")]
        web_root: PathBuf,
    },
}
