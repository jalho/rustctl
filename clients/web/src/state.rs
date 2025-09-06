use dioxus::prelude::*;
use rustctl_common::{
    in_game_events::{InGameEvent, Resource},
    snapshot::Snapshot,
};
use std::collections::HashMap;

#[derive(Clone, serde::Serialize, PartialEq)]
pub struct State {
    pub snapshot: Snapshot,
    pub aggregated: HashMap<u64, HashMap<Resource, f64>>,
}

pub static LATEST_SNAPSHOT: GlobalSignal<Option<State>> = GlobalSignal::new(|| None);

pub fn handle_snapshot(snapshot: Snapshot) {
    LATEST_SNAPSHOT.with_mut(|slot| {
        let aggregated = slot.as_ref().map(|s| s.aggregated.clone()).unwrap_or_default();
        *slot = Some(State { snapshot, aggregated });
    });
}

pub fn handle_incremental_event(event: InGameEvent) {
    LATEST_SNAPSHOT.with_mut(|slot| {
        if let Some(state) = slot {
            let mut aggregated = state.aggregated.clone();

            match event {
                InGameEvent::OnDispenserGather {
                    steam_id,
                    amount,
                    resource,
                }
                | InGameEvent::OnDispenserBonus {
                    steam_id,
                    amount,
                    resource,
                }
                | InGameEvent::OnGrowableGathered {
                    steam_id,
                    amount,
                    resource,
                } => {
                    let player_map = aggregated.entry(steam_id).or_default();
                    *player_map.entry(resource).or_insert(0.0) += amount;
                }
                InGameEvent::OnCollectiblePickup { steam_id, items } => {
                    let player_map = aggregated.entry(steam_id).or_default();
                    for item in items {
                        *player_map.entry(item.resource).or_insert(0.0) += item.amount;
                    }
                }
                InGameEvent::OnCargoShipSpawnCrate => {}
                _ => {}
            }

            *state = State {
                snapshot: state.snapshot.clone(),
                aggregated,
            };
        }
    });
}

pub fn clear_state() {
    LATEST_SNAPSHOT.with_mut(|slot| *slot = None);
}
