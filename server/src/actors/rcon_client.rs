pub struct RconClient {
    ctoken: tokio_util::sync::CancellationToken,
    tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

    cfg_client: crate::storage::GameServerConfigurationShared,

    /// "IGS" = "In-Game State"
    tx_agg_igs: tokio::sync::mpsc::Sender<rustctl_common::snapshot::InGameStateExposed>,
}

impl RconClient {
    pub fn new(
        ctoken: tokio_util::sync::CancellationToken,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

        cfg_client: crate::storage::GameServerConfigurationShared,

        tx_agg_igs: tokio::sync::mpsc::Sender<rustctl_common::snapshot::InGameStateExposed>,
    ) -> Self {
        Self {
            ctoken,
            tx_activate,

            cfg_client,

            tx_agg_igs,
        }
    }

    pub async fn work(self) -> Summary {
        const RECONNECT_DELAY: std::time::Duration = std::time::Duration::from_secs(5);

        'reconnect: loop {
            tokio::time::sleep(RECONNECT_DELAY).await;

            let connection_string: String = self.cfg_client.get_config().await.get_rcon_connection_string();
            let websocket: WebSocket = match tokio_tungstenite::connect_async(connection_string).await {
                Ok(n) => {
                    log::info!("RCON client connected");
                    let (websocket, _response): (WebSocket, Response) = n;
                    websocket
                }
                Err(_err) => {
                    continue 'reconnect;
                }
            };

            let (ws_sink, ws_stream): (WebSocketSink, WebSocketStream) = futures_util::StreamExt::split(websocket);

            Self::loop_query_rcon(ws_sink, ws_stream, self.tx_agg_igs.clone()).await;
        }
    }

    async fn loop_query_rcon(
        mut ws_sink: WebSocketSink,
        ws_stream: WebSocketStream,
        tx_agg_igs: tokio::sync::mpsc::Sender<rustctl_common::snapshot::InGameStateExposed>,
    ) -> () {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        'query: loop {
            interval.tick().await;

            let cmd: RconCommand = RconCommand::env_time();
            let cmd_serialized: String =
                serde_json::to_string(&cmd).expect("infallible: RconCommand should be serializable as JSON");
            let cmd_msg: tokio_tungstenite::tungstenite::Message =
                tokio_tungstenite::tungstenite::Message::Text(cmd_serialized.into());

            if let Err(err) = futures_util::SinkExt::send(&mut ws_sink, cmd_msg).await {
                log::error!("Failed to send RCON command: {err}");
                break 'query;
            };

            todo!("await response for the sent RCON command, then deserialize and send over `tx_agg_igs`");
        }
    }
}

pub struct Summary;

type WebSocket = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
type WebSocketSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::Message,
>;
type WebSocketStream = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
>;

type Response = axum::http::Response<Option<Vec<u8>>>;

#[derive(Debug, serde::Serialize)]
#[allow(non_snake_case)]
struct RconCommand {
    Identifier: u32,
    Message: String,
}

impl RconCommand {
    pub fn env_time() -> Self {
        Self {
            Identifier: Self::generate_message_identifier(),
            Message: "env.time".to_owned(),
        }
    }

    fn generate_message_identifier() -> u32 {
        let mut rng = rand::rng();
        let message_id: u32 = rand::Rng::random_range(&mut rng, 1..=u32::MAX);
        message_id
    }
}

#[derive(Debug, serde::Deserialize)]
#[allow(non_snake_case)]
struct RconResponse {
    Identifier: u32,
    Message: String,
}
