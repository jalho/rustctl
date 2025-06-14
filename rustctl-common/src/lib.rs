pub mod snapshot {
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Snapshot {
        pub clients_connected_all: std::collections::HashMap<uuid::Uuid, ClientExposed>,

        pub read_finished_at: chrono::DateTime<chrono::Utc>,
        pub read_duration_ns: u128,

        pub game: Game,
        pub system: System,
    }

    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    pub struct ClientExposed {
        pub id: uuid::Uuid,
        pub connected_at: chrono::DateTime<chrono::Utc>,
        pub addr_hash: String,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub enum Game {
        Running {
            players: std::collections::HashMap<Identifier, Player>,
            toolcupboards: std::collections::HashMap<Identifier, Toolcupboard>,
        },
    }

    #[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
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

    #[derive(serde::Serialize, serde::Deserialize)]
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

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Player {
        /// Steam ID.
        pub id: Identifier,
        pub location: Location,
        /*
         * TODO: Add rotation information of player (i.e. which direction
         *       they're looking at).
         */
    }

    #[derive(serde::Serialize, serde::Deserialize)]
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
