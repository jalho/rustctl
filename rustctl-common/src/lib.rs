pub mod snapshot {
    use crate::state_machine;

    /// Snapshot of the remote (server) state sent to each client (on a regular
    /// interval).
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Snapshot {
        pub captured_at: chrono::DateTime<chrono::Utc>,

        /// ID of the snapshot receiving client.
        pub client_id: uuid::Uuid,
        /// Salted hash of the IP address of the snapshot receiving client.
        pub ip_hash_salted: String,

        pub clients_connected_all: std::collections::HashMap<uuid::Uuid, ClientExposed>,

        pub game: Game,
        pub system: System,
    }

    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    pub struct ClientExposed {
        pub id: uuid::Uuid,
        pub connected_at: chrono::DateTime<chrono::Utc>,
        pub ip_hash_salted: String,
    }

    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    pub enum Game {
        Init(state_machine::Init),
        NotRunning(state_machine::NotRunning),
        StartupInProgress(state_machine::StartupInProgress),
        RunningHealthy(state_machine::RunningHealthy),
        ShutdownInProgress(state_machine::ShutdownInProgress),
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
    pub struct Identifier(String);

    impl Identifier {
        pub fn new(id: &str) -> Option<Self> {
            if id.len() > 0 {
                Some(Self(id.to_owned()))
            } else {
                None
            }
        }
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct Location {
        /*
         * TODO: What's the resolution? Meters? Centimeters? Define as some
         *       signed int...
         */
        /// From the center of the map to east (or west, if negative), in <TODO: UNIT>.
        pub x: (),
        /// From the center of the map to north (or south, if negative), in <TODO: UNIT>.
        pub y: (),
        /// From the center of the map sea level to up (or down, if negative), in <TODO: UNIT>.
        pub z: (),
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct Player {
        /// Steam ID.
        pub id: Identifier,
        pub location: Location,
        /*
         * TODO: Add rotation information of player (i.e. which direction
         *       they're looking at).
         */
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct Toolcupboard {
        /// In-game identifier of the game world object.
        pub id: Identifier,
        pub location: Location,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct System {
        /*
         * TODO: Define units...
         */
        pub cpu: (),
        pub memory: (),
    }
}

pub mod web_app {
    pub const WEBSOCKET_CONNECT_URL_PATH: &'static str = "/api/websocket";
}

pub mod state_machine {
    use crate::snapshot::{Identifier, Player, Toolcupboard};

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct Init {
        pub state_transitioned_into_at: chrono::DateTime<chrono::Utc>,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct NotRunning {
        pub state_transitioned_into_at: chrono::DateTime<chrono::Utc>,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct StartupInProgress {
        pub state_transitioned_into_at: chrono::DateTime<chrono::Utc>,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct Shutoff {
        pub state_transitioned_into_at: chrono::DateTime<chrono::Utc>,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ShutdownInProgress {
        pub state_transitioned_into_at: chrono::DateTime<chrono::Utc>,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct RunningHealthy {
        pub state_transitioned_into_at: chrono::DateTime<chrono::Utc>,
        players: std::collections::HashMap<Identifier, Player>,
        toolcupboards: std::collections::HashMap<Identifier, Toolcupboard>,
    }
}
