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

        /*
         * TODO: Investigate: Why does the RCON WebSocket connection keep dropping?
         *
         * $ grep "RCON client connected" out.log
         * 2025-08-27 18:21:46 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:21:54 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:21:59 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:22:04 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:22:09 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:22:14 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:22:20 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:22:26 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:22:33 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:22:38 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:22:43 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:22:48 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:22:54 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:22:59 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:23:04 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:23:09 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:23:14 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:23:19 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:23:24 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:23:30 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:23:35 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:23:40 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:23:46 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:23:51 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:23:56 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:24:01 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:24:06 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:24:11 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:24:16 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:24:21 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:24:26 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:24:31 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:24:36 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:24:41 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:24:46 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         * 2025-08-27 18:24:52 [rustctl] RCON client connected [server/src/actors/rcon_client.rs:39]
         */

        /*
         * TODO: Run only until canceled! (Use self.ctoken: tokio_util::sync::CancellationToken)
         */

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
        mut ws_stream: WebSocketStream,
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

            // TODO: Add timeout for waiting for the response!
            let response: RconResponse = match Self::wait_response(&cmd, &mut ws_stream).await {
                Some(n) => n,
                None => {
                    break 'query;
                }
            };
            // TODO: Send the queried in-game state to aggregator over `tx_agg_igs`
            dbg!(response);
        }
    }

    async fn wait_response(command: &RconCommand, ws_stream: &mut WebSocketStream) -> Option<RconResponse> {
        'collect_response: loop {
            let msg: tokio_tungstenite::tungstenite::Message = match futures_util::StreamExt::next(ws_stream).await {
                Some(Ok(msg)) => msg,
                Some(Err(err)) => todo!(),
                None => todo!(),
            };
            let msg: String = match &msg {
                tokio_tungstenite::tungstenite::Message::Text(utf8_bytes) => utf8_bytes.to_string(),
                tokio_tungstenite::tungstenite::Message::Binary(_)
                | tokio_tungstenite::tungstenite::Message::Ping(_)
                | tokio_tungstenite::tungstenite::Message::Pong(_)
                | tokio_tungstenite::tungstenite::Message::Close(_)
                | tokio_tungstenite::tungstenite::Message::Frame(_) => {
                    log::error!("Received a non-text message from RCON WebSocket: {msg:?}");
                    return None;
                }
            };
            let rcon_msg: RconResponse = match serde_json::from_str(&msg) {
                Ok(n) => n,
                Err(err) => todo!(),
            };

            log::debug!("Received RCON message: {rcon_msg:?}");

            if rcon_msg.Identifier == command.Identifier {
                return Some(rcon_msg);
            } else {
                continue 'collect_response;
            }
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
    Identifier: i32,
    Message: String,
}

impl RconCommand {
    pub fn env_time() -> Self {
        Self {
            Identifier: Self::generate_message_identifier(),
            Message: "env.time".to_owned(),
        }
    }

    /// The RCON identifier must presumably fit in a signed 32-bit integer.
    ///
    /// Evidence: Error seen in `RustDedicated` buildid `19600410` (latest as
    /// of 2025-08-27):
    /// ```
    /// JsonReaderException: JSON integer 3921165172 is too large or small for an Int32. Path 'Identifier', line 1, position 24.
    /// ```
    fn generate_message_identifier() -> i32 {
        let mut rng = rand::rng();
        let message_id: i32 = rand::Rng::random_range(&mut rng, 1..=i32::MAX);
        message_id
    }
}

#[derive(Debug, serde::Deserialize)]
#[allow(non_snake_case)]
struct RconResponse {
    Identifier: i32,
    Message: String,
}
