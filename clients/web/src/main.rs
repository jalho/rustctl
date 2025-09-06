use dioxus::prelude::*;
use futures_util::StreamExt;
use gloo_net::websocket::{Message, futures::WebSocket};
use rustctl_common::{BroadcastMessage, in_game_events::InGameEvent, snapshot::Snapshot};
use wasm_bindgen_futures::spawn_local;

#[derive(serde::Serialize)]
struct State {
    snapshot: Snapshot,
    /*
     * TODO: Aggregate from BroadcastMessage::EventIncremental: Keep a HashMap
     *       of amounts per player (steam_id) per resource
     */
    aggregated: (),
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
                                        /*
                                         * TODO: Only assign snapshot, leave aggregated intact
                                         */
                                        LATEST_SNAPSHOT.with_mut(|slot| {
                                            *slot = Some(State {
                                                snapshot,
                                                aggregated: (),
                                            })
                                        });
                                    }
                                    BroadcastMessage::EventIncremental(in_game_event) => {
                                        /*
                                         * TODO: Aggregate to state.aggregated
                                         */
                                        let event: InGameEvent = in_game_event;
                                        todo!()
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

    rsx! {
        div {
            h1 { "WebSocket JSON Viewer" }
            CodeView {}
            h2 { "Game World Map" }
            MapView {}
        }
    }
}

#[component]
fn CodeView() -> Element {
    let payload = LATEST_SNAPSHOT.read();
    match &*payload {
        Some(state) => {
            if let Ok(pretty) = serde_json::to_string_pretty(state) {
                rsx!(
                    pre { "{pretty}" }
                )
            } else {
                rsx!(
                    p { "Failed to render snapshot" }
                )
            }
        }
        None => rsx!(
            p { "Waiting for messages..." }
        ),
    }
}

const WORLD_MAP_RENDER_MARGIN: f64 = 1000.0;

#[component]
fn MapView() -> Element {
    let payload = LATEST_SNAPSHOT.read();
    let state: &State = match &*payload {
        Some(s) => s,
        None => {
            return rsx!(
                p { "No map data yet" }
            );
        }
    };

    let map_width = 800.0;
    let map_height = 800.0;

    let world_size = state.snapshot.game_world_size + WORLD_MAP_RENDER_MARGIN;
    let world_half = world_size / 2.0;

    rsx! {
        div { style: "position: relative; width: {map_width}px; height: {map_height}px; border: 1px solid black;",
            img {
                src: format!("{}{}", BACKEND_URL, rustctl_common::web_app::MAP_URL_PATH),
                alt: "Current game world map",
                style: "width: 100%; height: 100%; display: block;",
            }
            for player in &state.snapshot.ingame_state.players_pos {
                {
                    let (x, _y, z) = player.position;
                    let left = ((x + world_half) / world_size * map_width) as f64;
                    let top = ((world_half - z) / world_size * map_height) as f64;
                    rsx! {
                        div {
                            style: "position: absolute; left: {left}px; top: {top}px; width: 10px; height: 10px; \
                                                            background: red; border-radius: 50%; transform: translate(-50%, -50%);",
                            title: "{player.display_name}",
                        }
                    }
                }
            }
        }
    }
}
