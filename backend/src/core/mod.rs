use crate::{constants::INTERVAL_SYNC_CLIENT, game::GameState, system::SystemState};
use axum::{
    extract::{
        ConnectInfo, State, WebSocketUpgrade,
        ws::{Message, Utf8Bytes, WebSocket},
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, MutexGuard};

pub async fn handle_websocket_upgrade(
    State(state): State<Arc<Mutex<SharedState>>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |sock| {
        let client = Client::new(addr, sock, Arc::clone(&state));
        client.send_and_receive_messages()
    })
}

#[derive(serde::Serialize)]
struct ClientState {
    pub game: GameState,
    pub system: SystemState,
}

impl From<&SharedState> for ClientState {
    fn from(value: &SharedState) -> Self {
        Self {
            game: value.game.clone(),
            system: value.system.clone(),
        }
    }
}

pub struct SharedState {
    pub game: GameState,
    pub system: SystemState,
}

impl SharedState {
    pub fn init() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            game: GameState::read(),
            system: SystemState::read(),
        }))
    }

    pub fn serialize(&self) -> Message {
        let payload: ClientState = self.into();
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

                    {
                        let shared_locked: MutexGuard<SharedState> = shared_tx.lock().await;
                        let sent = SinkExt::send(&mut sock_tx, shared_locked.serialize()).await;
                        if sent.is_err() {
                            break;
                        }
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
        return <Cli as clap::Parser>::parse();
    }
}

#[derive(clap::Subcommand)]
pub enum CliCommand {
    Start {
        #[arg(long = "web-root", short, value_name = "PATH")]
        web_root: PathBuf,
    },
}
