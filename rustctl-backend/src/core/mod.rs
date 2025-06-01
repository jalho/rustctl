use crate::constants::INTERVAL_SYNC_CLIENT;
use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;

#[derive(Clone)]
pub struct CrossTasksSharedState {
    /*
     * TODO: Define a cross-tasks shared state...
     */
}

impl CrossTasksSharedState {
    pub fn init() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {}))
    }
}

pub struct Client {
    id: Uuid,
    connected_at: chrono::DateTime<chrono::Utc>,
    addr: SocketAddr,
    sock: WebSocket,
    shared: Arc<Mutex<CrossTasksSharedState>>,
}

impl Client {
    pub fn new(
        connected_at: chrono::DateTime<chrono::Utc>,
        addr: SocketAddr,
        sock: WebSocket,
        shared: Arc<Mutex<CrossTasksSharedState>>,
    ) -> Self {
        Self {
            addr,
            sock,
            shared,
            id: Uuid::new_v4(),
            connected_at,
        }
    }

    pub async fn send_and_receive_messages(self) {
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
                            println!("TODO: Got a message: \"{msg}\"");
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
                let mut interval = tokio::time::interval(INTERVAL_SYNC_CLIENT);
                loop {
                    interval.tick().await;

                    let sendable: rustctl_common::snapshot::Snapshot;
                    let snapshot: CrossTasksSharedState;
                    {
                        let shared_locked: MutexGuard<CrossTasksSharedState> =
                            shared_tx.lock().await;
                        snapshot = shared_locked.clone();
                    }
                    sendable = snapshot.into();

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
    }
}

impl From<CrossTasksSharedState> for rustctl_common::snapshot::Snapshot {
    fn from(_value: CrossTasksSharedState) -> Self {
        Self {
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
