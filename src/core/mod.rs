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
                    let command = Command::new(&msg.to_string());

                    // store command in short log in-mem
                    {
                        let command = command.clone();
                        let mut shared = shared_rx.lock().await;
                        match shared.clients.get_mut(&addr) {
                            Some(initalized) => {
                                if initalized.commands.len() >= MESSAGES_PER_CLIENT_INMEM_MAX {
                                    initalized.commands.pop_front();
                                }
                                initalized.commands.push_back(command);
                            }
                            None => {
                                let mut client = Client::new();
                                client.commands.push_front(command);
                                shared.clients.insert(addr, client);
                            }
                        }
                    }

                    // TODO: Do a state transition based on the received command
                    println!("TODO: Transition state: {command:?}");
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

#[derive(serde::Serialize)]
pub struct Client {
    commands: VecDeque<Command>,
}

impl Client {
    pub fn new() -> Self {
        Self {
            commands: VecDeque::new(),
        }
    }
}
