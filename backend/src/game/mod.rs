use crate::{
    constants::INTERVAL_FETCH_GAME_STATE,
    core::{Command, SharedState},
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::Mutex;

pub async fn read_state(shared: Arc<Mutex<SharedState>>) {
    let mut interval = tokio::time::interval(INTERVAL_FETCH_GAME_STATE);
    loop {
        interval.tick().await;

        let state = GameState::read();

        {
            let mut shared = shared.lock().await;
            shared.game = state;
        }
    }
}

#[derive(serde::Serialize)]
#[serde(tag = "_type", content = "data")]
pub enum GameState {
    Installing {
        commands_available: HashSet<Command>,
    },

    StartupInProgress {
        commands_available: HashSet<Command>,
    },

    Running {
        commands_available: HashSet<Command>,

        /// Time of day in the game world.
        time_of_day: f64,

        players: HashMap<Identifier, Player>,

        toolcupboards: HashMap<Identifier, Toolcupboard>,
    },
}

impl GameState {
    pub fn read() -> Self {
        // TODO: Query game state via RCON
        let mut players = HashMap::new();
        let dummy_player = Player::dummy();
        players.insert(dummy_player.id.to_owned(), dummy_player);

        let mut commands_available = HashSet::new();
        commands_available.insert(Command::Stop);

        Self::Running {
            time_of_day: 0.0,
            players,
            toolcupboards: HashMap::new(),
            commands_available,
        }
    }
}

#[derive(serde::Serialize, Eq, PartialEq, Hash, Clone)]
pub struct Identifier(String);

#[derive(serde::Serialize)]
struct Coordinates {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(serde::Serialize)]
pub struct Toolcupboard {
    id: Identifier,
    coordinates: Coordinates,
}

/// ISO 3166-1 alpha-3
#[derive(serde::Serialize)]
enum CountryCodeIso3166_1Alpha3 {
    FIN,
}

#[derive(serde::Serialize)]
pub struct Player {
    id: Identifier,
    coordinates: Coordinates,
    display_name: String,
    country: CountryCodeIso3166_1Alpha3,
}

trait Dummy {
    fn dummy() -> Self;
}

impl Dummy for Player {
    fn dummy() -> Self {
        Self {
            id: Identifier("00000000000000000".into()),
            coordinates: Coordinates {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            display_name: "player123".into(),
            country: CountryCodeIso3166_1Alpha3::FIN,
        }
    }
}
