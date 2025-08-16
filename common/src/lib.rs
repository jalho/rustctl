pub mod snapshot {
    /// Snapshot of the remote (server) state sent to each client (on a regular
    /// interval).
    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    pub struct Snapshot {
        pub game_server_state: GameServerStateExposed,
        pub system_memory_usage_total: TimedValue<MemoryUsage>,
        pub system_cpu_usage_total: TimedValue<Vec<CpuUsage>>,
    }

    /// A single CPU's utilization rate in some time window.
    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    pub struct CpuUsage(f64);

    impl CpuUsage {
        pub fn new(value: f64) -> Self {
            assert!((0.0..=100.0).contains(&value), "{value}");
            Self(value)
        }

        pub fn as_percentage(&self) -> f64 {
            self.0
        }
    }

    impl std::fmt::Display for CpuUsage {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{} %", self.0)
        }
    }

    /// Amount of memory used, in kibiytes (KiB, i.e. 1024 bytes).
    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    pub struct MemoryUsage(u64);

    impl MemoryUsage {
        pub fn new(value: u64) -> Self {
            Self(value)
        }
    }

    impl std::fmt::Display for MemoryUsage {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{kib} KiB (~ {gib:.2} GiB = {gb:.2} GB)",
                kib = self.0,
                gib = self.0 as f32 / 1_048_576.0,
                gb = (self.0 * 1024) as f32 / 1_000_000_000.0
            )
        }
    }

    impl Snapshot {
        pub fn init() -> Self {
            let timestamp = chrono::Utc::now();
            Self {
                game_server_state: GameServerStateExposed::Init,
                system_memory_usage_total: TimedValue {
                    read_completed_by: timestamp,
                    read_value: MemoryUsage(0),
                },
                system_cpu_usage_total: TimedValue {
                    read_completed_by: timestamp,
                    read_value: Vec::new(),
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
    pub struct GameServerMetaExposed {
        pub buildid: u32,
    }

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub enum GameServerStateExposed {
        Init,
        InstallingUpdates,
        InstalledAndConfigured { game_meta: GameServerMetaExposed },
        LaunchingGame { game_meta: GameServerMetaExposed },
        GameRunningHealthy { game_meta: GameServerMetaExposed },
        SavingAndClosingGame { game_meta: GameServerMetaExposed },
        GameClosedManually,
        GameTerminatedUnexpectedly,
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
