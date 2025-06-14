use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use log4rs::{append::console::ConsoleAppender, config::Appender, encode::pattern::PatternEncoder};
use rustctl_common::snapshot::ClientExposed;
use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;

#[derive(Clone)]
pub struct CrossTasksSharedState {
    pub clients_connected_all: HashMap<Uuid, ClientExposed>,
}

impl CrossTasksSharedState {
    pub fn init() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            clients_connected_all: HashMap::new(),
        }))
    }
}

/// An alive WebSocket connection to a client.
pub struct Client {
    pub id: Uuid,
    connected_at: chrono::DateTime<chrono::Utc>,
    addr: SocketAddr,
    sock: WebSocket,

    /// Handle to the state that is shared between many concurrent WebSockets.
    shared: Arc<Mutex<CrossTasksSharedState>>,
}

impl Into<ClientExposed> for &Client {
    fn into(self) -> ClientExposed {
        ClientExposed {
            id: self.id,
            connected_at: self.connected_at,
            addr_hash: format!("TODO: hash address: {}", self.addr),
        }
    }
}

impl Client {
    pub async fn new(
        connected_at: chrono::DateTime<chrono::Utc>,
        addr: SocketAddr,
        sock: WebSocket,
        shared: Arc<Mutex<CrossTasksSharedState>>,
    ) -> Self {
        let id = Uuid::new_v4();
        let client = Self {
            addr,
            sock,
            shared: shared.clone(),
            id,
            connected_at,
        };
        let client_exposed: ClientExposed = (&client).into();

        {
            let mut lock = shared.lock().await;
            lock.clients_connected_all.insert(id, client_exposed);
        }
        log::info!("Client registered: {id}: {addr}");

        return client;
    }

    pub async fn send_and_receive_messages(self, interval: Duration) {
        let (mut sock_tx, mut sock_rx) = StreamExt::split(self.sock);

        let _shared_rx: Arc<Mutex<CrossTasksSharedState>> = Arc::clone(&self.shared);
        let mut task_rx_cmd = tokio::task::Builder::new()
            .name("recv_commands")
            .spawn(async move {
                loop {
                    let recv = StreamExt::next(&mut sock_rx).await;

                    match recv {
                        Some(Ok(Message::Text(msg))) => {
                            // TODO: Do a state transition based on the received command?
                            log::debug!("TODO: Got a message: \"{msg}\"");
                        }
                        _ => {
                            break;
                        }
                    }
                }
            })
            .unwrap();

        let shared_tx: Arc<Mutex<CrossTasksSharedState>> = Arc::clone(&self.shared);
        let mut task_tx_state = tokio::task::Builder::new()
            .name("send_state")
            .spawn(async move {
                let mut interval = tokio::time::interval(interval);
                loop {
                    interval.tick().await;

                    let snapshot: CrossTasksSharedState;
                    {
                        let shared_locked: MutexGuard<CrossTasksSharedState> =
                            shared_tx.lock().await;
                        snapshot = shared_locked.clone();
                    }

                    let sendable: rustctl_common::snapshot::Snapshot = snapshot.into();
                    let serialized: String = serde_json::to_string(&sendable).unwrap();

                    let sent = SinkExt::send(&mut sock_tx, serialized.into()).await;
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

        {
            let mut lock = self.shared.lock().await;
            lock.clients_connected_all.remove(&self.id);
        }
        log::info!(
            "Client unregistered: {id}: {addr}",
            id = self.id,
            addr = self.addr
        );
    }
}

impl From<CrossTasksSharedState> for rustctl_common::snapshot::Snapshot {
    fn from(value: CrossTasksSharedState) -> Self {
        Self {
            clients_connected_all: value.clients_connected_all,

            game: rustctl_common::snapshot::Game::Running {
                players: HashMap::new(),
                toolcupboards: HashMap::new(),
            },
            system: rustctl_common::snapshot::System {
                cpu: (),
                memory: (),
            },

            /*
             * TODO: Make Snapshot not with `From` trait, but instead using
             *       some parameterized method that takes the read instant and
             *       duration...
             */
            read_finished_at: chrono::Utc::now(),
            read_duration_ns: 0,
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
        #[arg(long = "listen-addr", short = 'l', value_name = "ADDR")]
        listen_addr: SocketAddr,

        #[arg(long = "cors-allow-origin", short = 'O', value_name = "ORIGIN")]
        cors_allow_origin: String,

        #[arg(long = "tls-key-pem", short = 'k', value_name = "PATH")]
        tls_key_pem: Option<PathBuf>,

        #[arg(long = "tls-cert-pem", short = 'c', value_name = "PATH")]
        tls_cert_pem: Option<PathBuf>,
    },
}

pub fn init_logging(level: log::LevelFilter) -> log4rs::Handle {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{h({d(%Y-%m-%dT%H:%M:%SZ)(utc)} {l} - {m})} [{f}:{L}] [{T}]\n",
        )))
        .build();

    let name = "stdout";

    let config = log4rs::Config::builder()
        .appender(Appender::builder().build(name, Box::new(stdout)))
        .build(log4rs::config::Root::builder().appender(name).build(level))
        .unwrap();

    log4rs::init_config(config).unwrap()
}
