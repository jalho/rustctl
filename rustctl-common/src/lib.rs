pub mod snapshot {
    /// Snapshot of the remote (server) state sent to each client (on a regular
    /// interval).
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Snapshot {
        pub captured_at: chrono::DateTime<chrono::Utc>,
        pub game_server_state: GameServerStateExposed,
    }

    impl Snapshot {
        pub fn dummy() -> Self {
            let timestamp = chrono::Utc::now();
            Self {
                captured_at: timestamp,
                game_server_state: GameServerStateExposed::NotRunning(TrackedState {
                    transitioned_into_at: timestamp,
                    value: (),
                }),
            }
        }
    }

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub struct TrackedState<T> {
        pub transitioned_into_at: chrono::DateTime<chrono::Utc>,
        pub value: T,
    }

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub enum GameServerStateExposed {
        NotRunning(TrackedState<()>),
        Wiping(TrackedState<()>),
        Updating(TrackedState<()>),
        Launching(TrackedState<()>),
        RunningHealthy(TrackedState<()>),
        Stopping(TrackedState<()>),
    }
}

pub mod web_app {
    pub const WEBSOCKET_CONNECT_URL_PATH: &str = "/api/websocket";
}

pub mod command {
    #[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
    pub enum DownstreamClientMessage {
        ServerSaveAndClose,
        ServerConfigure { cfg: GameServerConfigurationPatch },
        ServerInstallOrUpdateAndStart,
        GameWorldKillPlayer { id: String },
        WebSocketProtocolOther,
    }

    impl TryFrom<&axum::extract::ws::Message> for DownstreamClientMessage {
        type Error = serde_json::Error;

        fn try_from(value: &axum::extract::ws::Message) -> Result<Self, Self::Error> {
            let utf8: String = match value {
                axum::extract::ws::Message::Text(utf8_bytes) => utf8_bytes.to_string(),
                axum::extract::ws::Message::Binary(_)
                | axum::extract::ws::Message::Ping(_)
                | axum::extract::ws::Message::Pong(_)
                | axum::extract::ws::Message::Close(_) => return Ok(Self::WebSocketProtocolOther),
            };
            let message: DownstreamClientMessage = serde_json::from_str(&utf8)?;
            Ok(message)
        }
    }

    #[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
    pub struct GameServerConfigurationPatch {}
}
