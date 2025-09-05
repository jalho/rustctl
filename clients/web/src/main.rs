use dioxus::prelude::*;
use futures_util::StreamExt;
use gloo_net::websocket::{Message, futures::WebSocket};
use rustctl_common::snapshot::Snapshot;
use wasm_bindgen_futures::spawn_local;

static LATEST_SNAPSHOT: GlobalSignal<Option<Snapshot>> = GlobalSignal::new(|| None);

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let interval = std::time::Duration::from_secs(1);

    use_effect(move || {
        spawn_local(async move {
            loop {
                let ws = WebSocket::open("ws://192.168.0.103:8080/api/websocket");
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
                            if let Ok(snapshot) = serde_json::from_str::<Snapshot>(&text) {
                                LATEST_SNAPSHOT.with_mut(|slot| *slot = Some(snapshot));
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
        Some(snapshot) => {
            if let Ok(pretty) = serde_json::to_string_pretty(snapshot) {
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

#[component]
fn MapView() -> Element {
    let payload = LATEST_SNAPSHOT.read();
    let snapshot = match &*payload {
        Some(s) => s,
        None => return rsx!( p { "No map data yet" } ),
    };

    let map_width = 800.0;
    let map_height = 800.0;
    let world_size = snapshot.game_world_size;
    let world_half = world_size / 2.0;

    rsx! {
        div {
            style: "position: relative; width: {map_width}px; height: {map_height}px; border: 1px solid black;",
            img {
                src: "http://192.168.0.103:8080/map",
                alt: "Current game world map",
                style: "width: 100%; height: 100%; display: block;",
            }
            for player in &snapshot.ingame_state.players_pos {
                {
                    let (x, _y, z) = player.position;

                    // Convert Rust world coordinates to map image pixels
                    let left = ((x + world_half) / world_size * map_width) as f64;
                    let top  = ((world_half - z) / world_size * map_height) as f64;

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

