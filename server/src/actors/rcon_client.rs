use futures_util::SinkExt;

pub struct RconClient {
    ctoken: tokio_util::sync::CancellationToken,
    cfg_client: crate::storage::GameServerConfigurationShared,
    /// "IGS" = "In-Game State"
    tx_agg_igs: tokio::sync::mpsc::Sender<rustctl_common::snapshot::InGameStateExposed>,
}

impl RconClient {
    pub fn new(
        ctoken: tokio_util::sync::CancellationToken,
        cfg_client: crate::storage::GameServerConfigurationShared,
        tx_agg_igs: tokio::sync::mpsc::Sender<rustctl_common::snapshot::InGameStateExposed>,
    ) -> Self {
        Self {
            ctoken,
            cfg_client,
            tx_agg_igs,
        }
    }

    pub async fn work(self) -> Summary {
        let ctoken = self.ctoken.child_token();
        let job = self.loop_reconnect();
        let done = ctoken.run_until_cancelled(job).await;
        if let Some(done) = done {
            let _done: () = done;
        }
        Summary {}
    }

    pub async fn loop_reconnect(self) -> () {
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

            if let Err(err) = Self::loop_query_rcon(ws_sink, ws_stream, self.tx_agg_igs.clone()).await {
                log::error!("Failed to query RCON: {err}");
                continue 'reconnect;
            }
        }
    }

    async fn loop_query_rcon(
        mut ws_sink: WebSocketSink,
        mut ws_stream: WebSocketStream,
        tx_agg_igs: tokio::sync::mpsc::Sender<rustctl_common::snapshot::InGameStateExposed>,
    ) -> Result<(), Error> {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            /*
             * env.time
             */
            let cmd: RconMessage = RconMessage::new("env.time");
            let response: RconMessage = cmd.send(&mut ws_sink, &mut ws_stream).await?;
            let env_time: rustctl_common::snapshot::EnvTime = (&response).try_into()?;

            /*
             * playerlistpos
             */
            let cmd: RconMessage = RconMessage::new("playerlistpos");
            let response: RconMessage = cmd.send(&mut ws_sink, &mut ws_stream).await?;
            /*
             * Example response: Case no players on the server -- Only header line:
             * ```
             * RconMessage {
             *     Identifier: 1000000001,
             *     Message: "SteamID DisplayName POS ROT \n",
             * }
             * ```
             */
            dbg!(response);
            // let todo: rustctl_common::snapshot::PlayerListPos = (&response).try_into()?;

            /*
             * playerlist
             */
            let cmd: RconMessage = RconMessage::new("playerlist");
            let response: RconMessage = cmd.send(&mut ws_sink, &mut ws_stream).await?;
            /*
             * Example response: Case no players on the server -- Empty JSON array:
             * ```
             * RconMessage {
             *     Identifier: 100000002,
             *     Message: "[]",
             * }
             * ```
             */
            dbg!(response);
            // let todo: rustctl_common::snapshot::PlayerList = (&response).try_into()?;

            /*
             * listtoolcupboards
             */
            let cmd: RconMessage = RconMessage::new("listtoolcupboards");
            let response: RconMessage = cmd.send(&mut ws_sink, &mut ws_stream).await?;
            /*
             * Example response: Case no toolcupboards on the server -- Only header line:
             * ```
             * RconMessage {
             *     Identifier: 1000000003,
             *     Message: "EntityId Position Authed \n",
             * }
             * ```
             */
            dbg!(response);
            // let todo: rustctl_common::snapshot::ListToolcupboards = (&response).try_into()?;

            let total = rustctl_common::snapshot::InGameStateExposed { env_time };

            if let Err(err) = tx_agg_igs.send(total).await {
                log::debug!(
                    "Channel for sending in-game state snapshots to aggregator closed -- Stopping querying: {err}"
                );
                return Ok(());
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[allow(non_snake_case)]
struct RconMessage {
    Identifier: i32,
    Message: String,
}

impl RconMessage {
    pub fn new(command: &str) -> Self {
        Self {
            Identifier: Self::generate_message_identifier(),
            Message: command.to_owned(),
        }
    }

    pub async fn send(
        &self,
        ws_sink: &mut WebSocketSink,
        ws_stream: &mut WebSocketStream,
    ) -> Result<RconMessage, Error> {
        let cmd_serialized: String =
            serde_json::to_string(&self).expect("infallible: RconCommand should be serializable as JSON");
        let cmd_msg: tokio_tungstenite::tungstenite::Message =
            tokio_tungstenite::tungstenite::Message::Text(cmd_serialized.into());

        if let Err(source) = ws_sink.send(cmd_msg).await {
            log::error!("Failed to send RCON command: {source}");
            return Err(Error::SocketFailed { source });
        };

        // TODO: Add timeout for waiting for the response!
        let response: RconMessage = match self.wait_response(ws_stream).await {
            Ok(n) => n,
            Err(err) => {
                log::error!("Error while waiting for RCON response: {err}");
                return Err(err);
            }
        };

        Ok(response)
    }

    async fn wait_response(&self, ws_stream: &mut WebSocketStream) -> Result<RconMessage, Error> {
        'collect_response: loop {
            let msg: tokio_tungstenite::tungstenite::Message = match futures_util::StreamExt::next(ws_stream).await {
                Some(Ok(msg)) => msg,
                Some(Err(source)) => return Err(Error::SocketFailed { source }),
                None => return Err(Error::SocketClosed),
            };
            let utf8_payload: String = match &msg {
                tokio_tungstenite::tungstenite::Message::Text(utf8_bytes) => utf8_bytes.to_string(),
                tokio_tungstenite::tungstenite::Message::Binary(_)
                | tokio_tungstenite::tungstenite::Message::Ping(_)
                | tokio_tungstenite::tungstenite::Message::Pong(_)
                | tokio_tungstenite::tungstenite::Message::Close(_)
                | tokio_tungstenite::tungstenite::Message::Frame(_) => {
                    log::error!("Received a non-text message from RCON WebSocket: {msg:?}");
                    return Err(Error::UnexpectedWebSocketMessage { msg });
                }
            };
            let rcon_msg: RconMessage = match serde_json::from_str(&utf8_payload) {
                Ok(n) => n,
                Err(source) => {
                    return Err(Error::InvalidRconMessage { source, utf8_payload });
                }
            };

            if rcon_msg.Identifier == self.Identifier {
                return Ok(rcon_msg);
            } else {
                continue 'collect_response;
            }
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

#[derive(Debug)]
enum Error {
    SocketFailed {
        source: tokio_tungstenite::tungstenite::Error,
    },

    SocketClosed,

    UnexpectedWebSocketMessage {
        msg: tokio_tungstenite::tungstenite::Message,
    },

    InvalidRconMessage {
        source: serde_json::Error,
        utf8_payload: String,
    },

    InvalidRconMessagePayload {
        rationale_display: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SocketFailed { source: _ } => write!(f, r#"socket failed"#),
            Error::SocketClosed => write!(f, r#"socket closed"#),
            Error::UnexpectedWebSocketMessage { msg } => write!(f, r#"unexpected WebSocket message: {msg:?}"#),
            Error::InvalidRconMessage {
                source: _,
                utf8_payload,
            } => write!(f, r#"invalid RCON message: "{utf8_payload}""#),
            Error::InvalidRconMessagePayload { rationale_display } => {
                write!(f, r#"invalid RCON message payload: {rationale_display}"#)
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::SocketFailed { source } => Some(source),
            Error::SocketClosed => None,
            Error::UnexpectedWebSocketMessage { msg: _ } => None,
            Error::InvalidRconMessage { source, .. } => Some(source),
            Error::InvalidRconMessagePayload { rationale_display: _ } => None,
        }
    }
}

impl TryFrom<&RconMessage> for rustctl_common::snapshot::EnvTime {
    type Error = Error;

    fn try_from(msg: &RconMessage) -> Result<Self, Self::Error> {
        let value: &String = &msg.Message;

        const PREFIX: &str = "env.time: ";
        if !value.starts_with(PREFIX) {
            return Err(Error::InvalidRconMessagePayload {
                rationale_display: format!(r#"invalid env.time format: expected "{PREFIX}" prefix, got "{value}""#),
            });
        }

        let quoted = &value[PREFIX.len()..];
        let unquoted = quoted.trim_matches('"').trim();

        match unquoted.parse::<f64>() {
            Ok(time_value) => Ok(rustctl_common::snapshot::EnvTime(time_value)),
            Err(err) => Err(Error::InvalidRconMessagePayload {
                rationale_display: format!(r#"failed to parse time value "{unquoted}": {err}"#),
            }),
        }
    }
}

impl TryFrom<&RconMessage> for Vec<rustctl_common::snapshot::PlayerPos> {
    type Error = Error;

    fn try_from(msg: &RconMessage) -> Result<Self, Self::Error> {
        let value: &String = &msg.Message;

        const HEADER: &str = "SteamID DisplayName POS ROT \n";
        if !value.starts_with(HEADER) {
            return Err(Error::InvalidRconMessagePayload {
                rationale_display: format!(
                    r#"invalid playerlistpos format: expected "{HEADER}" header, got "{value}""#
                ),
            });
        }

        let data_lines = &value[HEADER.len()..];
        let mut players = Vec::new();

        let re = regex::Regex::new(r#"(\d{17})(.*)\((.*)\)\s*\((.*)\)"#).map_err(|err| {
            Error::InvalidRconMessagePayload {
                rationale_display: format!(r#"failed to compile regex for playerlistpos parsing: {err}"#),
            }
        })?;

        for line in data_lines.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let captures = re.captures(line).ok_or_else(|| Error::InvalidRconMessagePayload {
                rationale_display: format!(r#"invalid playerlistpos line format: failed to match pattern in "{line}""#),
            })?;

            let steam_id = captures[1].to_string();
            let display_name = captures[2].trim().to_string();
            let position_raw = captures[3].to_string();
            let rotation_raw = captures[4].to_string();

            let pos_parts: Vec<&str> = position_raw.split(',').map(|s| s.trim()).collect();
            if pos_parts.len() != 3 {
                return Err(Error::InvalidRconMessagePayload {
                    rationale_display: format!(
                        r#"invalid position format: expected 3 coordinates, got "{position_raw}""#
                    ),
                });
            }

            let x: f64 = pos_parts[0].parse().map_err(|err| Error::InvalidRconMessagePayload {
                rationale_display: format!(r#"failed to parse x coordinate "{}" in "{line}": {err}"#, pos_parts[0]),
            })?;
            let y: f64 = pos_parts[1].parse().map_err(|err| Error::InvalidRconMessagePayload {
                rationale_display: format!(r#"failed to parse y coordinate "{}" in "{line}": {err}"#, pos_parts[1]),
            })?;
            let z: f64 = pos_parts[2].parse().map_err(|err| Error::InvalidRconMessagePayload {
                rationale_display: format!(r#"failed to parse z coordinate "{}" in "{line}": {err}"#, pos_parts[2]),
            })?;

            let rot_parts: Vec<&str> = rotation_raw.split(',').map(|s| s.trim()).collect();
            if rot_parts.len() != 3 {
                return Err(Error::InvalidRconMessagePayload {
                    rationale_display: format!(
                        r#"invalid rotation format: expected 3 coordinates, got "{rotation_raw}""#
                    ),
                });
            }

            let pitch: f64 = rot_parts[0].parse().map_err(|err| Error::InvalidRconMessagePayload {
                rationale_display: format!(r#"failed to parse pitch "{}" in "{line}": {err}"#, rot_parts[0]),
            })?;
            let yaw: f64 = rot_parts[1].parse().map_err(|err| Error::InvalidRconMessagePayload {
                rationale_display: format!(r#"failed to parse yaw "{}" in "{line}": {err}"#, rot_parts[1]),
            })?;
            let roll: f64 = rot_parts[2].parse().map_err(|err| Error::InvalidRconMessagePayload {
                rationale_display: format!(r#"failed to parse roll "{}" in "{line}": {err}"#, rot_parts[2]),
            })?;

            players.push(rustctl_common::snapshot::PlayerPos {
                steam_id,
                display_name,
                position: (x, y, z),
                rotation: (pitch, yaw, roll),
            });
        }

        Ok(players)
    }
}

impl TryFrom<&RconMessage> for Vec<rustctl_common::snapshot::Player> {
    type Error = Error;

    fn try_from(msg: &RconMessage) -> Result<Self, Self::Error> {
        let value: &String = &msg.Message;

        let players: Vec<rustctl_common::snapshot::Player> =
            serde_json::from_str(value).map_err(|source| Error::InvalidRconMessage {
                source,
                utf8_payload: value.clone(),
            })?;

        Ok(players)
    }
}

impl TryFrom<&RconMessage> for Vec<rustctl_common::snapshot::Toolcupboard> {
    type Error = Error;

    fn try_from(msg: &RconMessage) -> Result<Self, Self::Error> {
        let value: &String = &msg.Message;

        const HEADER: &str = "EntityId Position Authed \n";
        if !value.starts_with(HEADER) {
            return Err(Error::InvalidRconMessagePayload {
                rationale_display: format!(
                    r#"invalid listtoolcupboards format: expected "{HEADER}" header, got "{value}""#
                ),
            });
        }

        let data_lines = &value[HEADER.len()..];
        let mut toolcupboards = Vec::new();

        let re = regex::Regex::new(r#"(\d+)\s+\((.*)\)\s+(\d+)"#).map_err(|err| Error::InvalidRconMessagePayload {
            rationale_display: format!(r#"failed to compile regex for toolcupboard parsing: {err}"#),
        })?;

        for line in data_lines.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let captures = re.captures(line).ok_or_else(|| Error::InvalidRconMessagePayload {
                rationale_display: format!(
                    r#"invalid listtoolcupboards line format: failed to match pattern in "{line}""#
                ),
            })?;

            let entity_id = captures[1].to_string();
            let position_raw = captures[2].to_string();
            let auth_count_raw = captures[3].to_string();

            let pos_parts: Vec<&str> = position_raw.split(',').map(|s| s.trim()).collect();
            if pos_parts.len() != 3 {
                return Err(Error::InvalidRconMessagePayload {
                    rationale_display: format!(
                        r#"invalid position format: expected 3 coordinates, got "{position_raw}""#
                    ),
                });
            }

            let x: f64 = pos_parts[0].parse().map_err(|err| Error::InvalidRconMessagePayload {
                rationale_display: format!(r#"failed to parse x coordinate "{}" in "{line}": {err}"#, pos_parts[0]),
            })?;
            let y: f64 = pos_parts[1].parse().map_err(|err| Error::InvalidRconMessagePayload {
                rationale_display: format!(r#"failed to parse y coordinate "{}" in "{line}": {err}"#, pos_parts[1]),
            })?;
            let z: f64 = pos_parts[2].parse().map_err(|err| Error::InvalidRconMessagePayload {
                rationale_display: format!(r#"failed to parse z coordinate "{}" in "{line}": {err}"#, pos_parts[2]),
            })?;

            let auth_count: u32 = auth_count_raw.parse().map_err(|err| Error::InvalidRconMessagePayload {
                rationale_display: format!(r#"failed to parse auth count "{auth_count_raw}" in "{line}": {err}"#),
            })?;

            toolcupboards.push(rustctl_common::snapshot::Toolcupboard {
                entity_id,
                position: (x, y, z),
                auth_count,
            });
        }

        Ok(toolcupboards)
    }
}
