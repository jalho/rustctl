use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use log4rs::{append::console::ConsoleAppender, config::Appender, encode::pattern::PatternEncoder};
use rustctl_common::snapshot::{ClientExposed, Snapshot};
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
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

    /// Salted hash of the IP address of the hash, i.e. socket address without
    /// port number. Intended for identifying peer clients that belong to the
    /// same address without exposing the exact address.
    ip_hash_salted: String,

    /// Handle to the state that is shared between many concurrent WebSockets.
    shared: Arc<Mutex<CrossTasksSharedState>>,
}

impl std::fmt::Display for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let id_prefix: &str = &self.id.to_string()[..8];
        write!(
            f,
            "{id_prefix}: {addr} (ip_hash_salted {ip_hash_salted})",
            addr = self.addr,
            ip_hash_salted = self.ip_hash_salted,
        )
    }
}

impl Into<ClientExposed> for &Client {
    fn into(self) -> ClientExposed {
        ClientExposed {
            id: self.id,
            connected_at: self.connected_at,
            ip_hash_salted: self.ip_hash_salted.clone(),
        }
    }
}

impl Client {
    pub async fn new(
        connected_at: chrono::DateTime<chrono::Utc>,
        addr: SocketAddr,
        sock: WebSocket,
        shared: Arc<Mutex<CrossTasksSharedState>>,
        ip_hash_salt: &str,
    ) -> Self {
        let id = Uuid::new_v4();

        let addr_hash: String = hash_socket_ip_addr(addr.ip(), ip_hash_salt);

        let client = Self {
            addr,
            ip_hash_salted: addr_hash,
            sock,
            shared: shared.clone(),
            id,
            connected_at,
        };
        let client_exposed: ClientExposed = (&client).into();

        let clients_total: usize;
        {
            let mut lock = shared.lock().await;
            lock.clients_connected_all.insert(id, client_exposed);
            clients_total = lock.clients_connected_all.len();
        }
        log::info!("Client registered: {client} -- Total count: {clients_total}");

        return client;
    }

    pub async fn send_and_receive_messages(self, interval: Duration) {
        let self_display: String = self.to_string();
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
                    let captured_at: chrono::DateTime<chrono::Utc>;
                    {
                        let shared_locked: MutexGuard<CrossTasksSharedState> =
                            shared_tx.lock().await;
                        captured_at = chrono::Utc::now();
                        snapshot = shared_locked.clone();
                    }

                    let sendable: rustctl_common::snapshot::Snapshot = make_snapshot(
                        (self.id, self.ip_hash_salted.clone()),
                        (captured_at, snapshot),
                    );
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

        let clients_remaining: usize;
        {
            let mut lock = self.shared.lock().await;
            lock.clients_connected_all.remove(&self.id);
            clients_remaining = lock.clients_connected_all.len();
        }
        log::info!("Client unregistered: {self_display} -- Clients remaining: {clients_remaining}");
    }
}

fn hash_socket_ip_addr(addr: IpAddr, salt: &str) -> String {
    let mut hasher = DefaultHasher::new();
    salt.hash(&mut hasher);
    addr.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:x}", hash)[..8].to_string()
}

fn make_snapshot(
    for_client: (Uuid, String),
    from_state: (chrono::DateTime<chrono::Utc>, CrossTasksSharedState),
) -> Snapshot {
    let (client_id, ip_hash_salted) = for_client;
    let (captured_at, state) = from_state;
    let snapshot = Snapshot {
        captured_at,

        client_id,
        ip_hash_salted,
        clients_connected_all: state.clients_connected_all,

        game: rustctl_common::snapshot::Game::Running {
            players: HashMap::new(),
            toolcupboards: HashMap::new(),
        },
        system: rustctl_common::snapshot::System {
            cpu: (),
            memory: (),
        },
    };
    return snapshot;
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
