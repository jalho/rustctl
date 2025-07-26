pub mod snapshot {
    /// Snapshot of the remote (server) state sent to each client (on a regular
    /// interval).
    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    pub struct Snapshot {
        pub game_server_state: GameServerStateExposed,
        pub system_memory_usage_total: TimedValue<MemoryUsage>,
        pub system_cpu_usage_total: TimedValue<CpuUsage>,
    }

    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    pub struct CpuUsage(f32);

    impl CpuUsage {
        pub fn new(value: f32) -> Self {
            Self(value)
        }
    }

    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    pub struct MemoryUsage(u64);

    impl MemoryUsage {
        pub fn new(value: u64) -> Self {
            Self(value)
        }
    }

    impl Snapshot {
        pub fn init() -> Self {
            let timestamp = chrono::Utc::now();
            Self {
                game_server_state: GameServerStateExposed::NotRunning(TrackedState {
                    transitioned_into_at: timestamp,
                    value: (),
                }),
                system_memory_usage_total: TimedValue {
                    read_completed_by: timestamp,
                    read_value: MemoryUsage(0),
                },
                system_cpu_usage_total: TimedValue {
                    read_completed_by: timestamp,
                    read_value: CpuUsage(0.0),
                },
            }
        }
    }

    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    pub struct TimedValue<ReadValue> {
        pub read_completed_by: chrono::DateTime<chrono::Utc>,
        pub read_value: ReadValue,
    }

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub struct TrackedState<GameServerState> {
        pub transitioned_into_at: chrono::DateTime<chrono::Utc>,
        pub value: GameServerState,
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
