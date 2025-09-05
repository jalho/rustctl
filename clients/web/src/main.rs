use dioxus::prelude::*;
use futures_util::StreamExt;
use gloo_net::websocket::{Message, futures::WebSocket};
use rustctl_common::snapshot::Snapshot;
use wasm_bindgen_futures::spawn_local;

static LATEST_PAYLOAD: GlobalSignal<Option<String>> = GlobalSignal::new(|| None);

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
                                if let Ok(pretty) = serde_json::to_string_pretty(&snapshot) {
                                    LATEST_PAYLOAD.with_mut(|slot| {
                                        *slot = Some(pretty);
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }

                LATEST_PAYLOAD.with_mut(|slot| *slot = None);
                gloo_timers::future::sleep(interval).await;
            }
        });
    });

    rsx! {
        div {
            h1 { "WebSocket JSON Viewer" }
            CodeView {}
            h2 { "Game World Map" }
            img {
                src: "http://192.168.0.103:8080/map",
                alt: "Current game world map",
                style: "max-width: 100%;",
            }
        }
    }
}

#[component]
fn CodeView() -> Element {
    let payload = LATEST_PAYLOAD.read();
    match &*payload {
        Some(json) => rsx! {
            pre { "{json}" }
        },
        None => rsx! {
            p { "Waiting for messages..." }
        },
    }
}
