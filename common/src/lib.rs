pub mod rcon;
pub mod in_game_events;

pub mod snapshot {
    /// Snapshot of the remote (server) state sent to each client (on a regular
    /// interval).
    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    pub struct Snapshot {
        pub game_world_size: f64,
        pub ingame_state: InGameStateExposed,
        pub game_server_state: GameServerStateExposed,
        pub memory_used_kibibytes: MemoryUsage,
        pub cpus_utilization_percentage: Vec<CpuUsage>,
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
        pub fn init(game_world_size: f64) -> Self {
            Self {
                game_server_state: GameServerStateExposed::Init,
                memory_used_kibibytes: MemoryUsage(0),
                cpus_utilization_percentage: Vec::new(),
                ingame_state: InGameStateExposed::init(),
                game_world_size,
            }
        }
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
        SavingAndClosingGame {},
        GameClosedManually,
        GameTerminatedUnexpectedly,
    }

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub struct InGameStateExposed {
        pub env_time: crate::rcon::EnvTime,
        pub players_pos: Vec<crate::rcon::PlayerPos>,
        pub players: Vec<crate::rcon::Player>,
        pub toolcupboards: Vec<crate::rcon::Toolcupboard>,
    }

    impl InGameStateExposed {
        pub fn init() -> Self {
            Self {
                env_time: crate::rcon::EnvTime(0.0),
                players_pos: Vec::new(),
                players: Vec::new(),
                toolcupboards: Vec::new(),
            }
        }
    }
}

pub mod web_app {
    pub const WEBSOCKET_CONNECT_URL_PATH: &str = "/api/websocket";
    pub const MAP_URL_PATH: &str = "/api/map";
}

pub mod command {
    #[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
    pub enum DownstreamClientMessage {
        ServerSaveAndClose,
        ServerInstallOrUpdateAndStart,
        WebSocketProtocolOther,
    }
}
