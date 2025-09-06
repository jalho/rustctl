use dioxus::prelude::*;
use futures_util::StreamExt;
use gloo_net::websocket::{Message, futures::WebSocket};
use rustctl_common::{
    BroadcastMessage,
    in_game_events::{InGameEvent, Resource},
    snapshot::Snapshot,
};
use std::collections::HashMap;
use wasm_bindgen_futures::spawn_local;

mod components;
use components::{CodeView, MapView, AggregatedView};

#[derive(Clone, serde::Serialize, PartialEq)]
pub struct State {
    pub snapshot: Snapshot,
    pub aggregated: HashMap<u64, HashMap<Resource, f64>>,
}

static LATEST_SNAPSHOT: GlobalSignal<Option<State>> = GlobalSignal::new(|| None);

#[cfg(debug_assertions)]
const BACKEND_URL: &str = "http://192.168.0.103:8080";
#[cfg(debug_assertions)]
const WS_URL: &str = "ws://192.168.0.103:8080";

#[cfg(not(debug_assertions))]
const BACKEND_URL: &str = "https://rustctl.internal";
#[cfg(not(debug_assertions))]
const WS_URL: &str = "wss://rustctl.internal";

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let interval = std::time::Duration::from_secs(1);

    use_effect(move || {
        spawn_local(async move {
            loop {
                let ws = WebSocket::open(&format!(
                    "{}{}",
                    WS_URL,
                    rustctl_common::web_app::WEBSOCKET_CONNECT_URL_PATH
                ));
                let ws = match ws {
                    Ok(ws) => ws,
                    Err(_) => {
                        gloo_timers::future::sleep(interval).await;
                        continue;
                    }
                };

                let (_tx, mut rx) = ws.split();

                while let Some(msg) = rx.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Ok(msg) = serde_json::from_str::<BroadcastMessage>(&text) {
                                match msg {
                                    BroadcastMessage::Snapshot(snapshot) => {
                                        LATEST_SNAPSHOT.with_mut(|slot| {
                                            let aggregated =
                                                slot.as_ref().map(|s| s.aggregated.clone()).unwrap_or_default();
                                            *slot = Some(State { snapshot, aggregated });
                                        });
                                    }
                                    BroadcastMessage::EventIncremental(event) => {
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
                                                            *player_map.entry(item.resource).or_insert(0.0) +=
                                                                item.amount;
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
                                }
                            }
                        }
                        _ => {}
                    }
                }

                LATEST_SNAPSHOT.with_mut(|slot| *slot = None);
                gloo_timers::future::sleep(interval).await;
            }
        });
    });

    let state = LATEST_SNAPSHOT.read();

    rsx! {
        div {
            h1 { "WebSocket JSON Viewer" }
            CodeView { state: state.clone() }
            h2 { "Game World Map" }
            MapView { 
                state: state.clone(),
                backend_url: BACKEND_URL
            }
            h2 { "Aggregated Resources" }
            AggregatedView { state: state.clone() }
        }
    }
}
