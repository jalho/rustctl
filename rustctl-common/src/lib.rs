pub mod snapshot {
    /// Snapshot of the remote (server) state sent to each client (on a regular
    /// interval).
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Snapshot {
        pub captured_at: chrono::DateTime<chrono::Utc>,
        pub game_server_state: GameServerStateExposed,
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
    #[derive(serde::Serialize, serde::Deserialize)]
    pub enum Command {
        TransitionFromNotRunning,
        TransitionFromRunningHealthy,
    }
}
