use crate::{constants::INTERVAL_FETCH_GAME_STATE, core::SharedState};
use std::{collections::HashMap, sync::Arc};
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

/// State of the game (obtained via RCON).
#[derive(serde::Serialize)]
pub struct GameState {
    /// Time of day in the game world.
    time_of_day: f64,

    players: HashMap<Identifier, Player>,

    toolcupboards: HashMap<Identifier, Toolcupboard>,
}

impl GameState {
    pub fn read() -> Self {
        // TODO: Query game state via RCON
        Self {
            time_of_day: 0.0,
            players: HashMap::new(),
            toolcupboards: HashMap::new(),
        }
    }
}

#[derive(serde::Serialize)]
struct Identifier;

#[derive(serde::Serialize)]
struct Location;

#[derive(serde::Serialize)]
struct Toolcupboard {
    id: Identifier,
    location: Location,
}

#[derive(serde::Serialize)]
struct Player {
    id: Identifier,
    location: Location,
}
