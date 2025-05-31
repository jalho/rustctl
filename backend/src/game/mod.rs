use crate::{constants::INTERVAL_FETCH_GAME_STATE, core::SharedState};
use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
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

#[derive(serde::Serialize, Clone)]
#[serde(tag = "_type", content = "data")]
pub enum GameState {
    Running {
        /// When the state snapshotting was initiated.
        read_start_utc: DateTime<Utc>,

        /// How long the state snapshotting took, in nanoseconds.
        read_duration_ns: u128,

        /// Time of day in the game world.
        time_of_day: f64,

        players: HashMap<Identifier, Player>,
    },
}

impl GameState {
    pub fn read() -> Self {
        let mut players: HashMap<Identifier, Player>;
        let time_of_day: f64;

        /*
         * TODO: Query game state via RCON
         */
        let read_start = SystemTime::now();
        {
            time_of_day = 0.0;

            players = HashMap::new();
            let dummy_player = Player::dummy();
            players.insert(dummy_player.id.to_owned(), dummy_player);
        }
        let read_end = SystemTime::now();
        let elapsed: Duration = read_end.duration_since(read_start).unwrap();

        let read_start_utc: DateTime<Utc> = read_start.into();
        let read_duration_ns: u128 = elapsed.as_nanos();

        Self::Running {
            read_start_utc,
            read_duration_ns,

            time_of_day,
            players,
        }
    }
}

#[derive(serde::Serialize, Eq, PartialEq, Hash, Clone)]
pub struct Identifier(String);

#[derive(serde::Serialize, Clone)]
struct Coordinates {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(serde::Serialize, Clone)]
pub struct Toolcupboard {
    id: Identifier,
    coordinates: Coordinates,
}

/// ISO 3166-1 alpha-3
#[derive(serde::Serialize, Clone)]
#[allow(clippy::upper_case_acronyms)]
enum CountryCodeIso3166_1Alpha3 {
    FIN,
}

#[derive(serde::Serialize, Clone)]
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
