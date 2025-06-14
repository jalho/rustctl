use dioxus::prelude::*;
use futures_util::StreamExt;
use gloo_net::websocket::{Message, futures::WebSocket};
use rustctl_common::{snapshot::Snapshot, web_app::WEBSOCKET_CONNECT_URL_PATH};
use wasm_bindgen_futures::spawn_local;

static GLOBAL_SIGNAL: GlobalSignal<Option<Snapshot>> =
    GlobalSignal::<Option<Snapshot>>::new(|| None);

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let interval = std::time::Duration::from_secs(3);
    use_effect(move || {
        spawn_local(async move {
            'connect_websocket: loop {
                let ws_result = WebSocket::open(&format!(
                    "ws://localhost:8081{path}",
                    path = WEBSOCKET_CONNECT_URL_PATH
                ));

                let ws = match ws_result {
                    Ok(ws) => ws,
                    Err(_) => {
                        gloo_timers::future::sleep(interval).await;
                        continue 'connect_websocket;
                    }
                };

                let (_write, mut read) = ws.split();

                'recv_messages: while let Some(msg_result) = read.next().await {
                    match msg_result {
                        Ok(Message::Text(serialized)) => {
                            GLOBAL_SIGNAL.with_mut(|state| {
                                let deserialized: Snapshot =
                                    serde_json::from_str(&serialized).unwrap();
                                *state = Some(deserialized);
                            });
                        }
                        Ok(_) => {}
                        Err(_) => break 'recv_messages,
                    }
                }
                gloo_timers::future::sleep(interval).await;
            }
        });
    });

    rsx! {
        div {
            h1 { "WebSocket Message Viewer" }
            MessageView {}
        }
    }
}

#[component]
fn MessageView() -> Element {
    let value = GLOBAL_SIGNAL.read();

    match *value {
        Some(ref n) => {
            let snapshot: &Snapshot = n;
            let serialized: String = serde_json::to_string_pretty(snapshot).unwrap();
            rsx! {
                div {
                    h2 { "Latest message:" }
                    pre { "{serialized}" }
                }
            }
        }
        None => {
            rsx! {
                div {
                    p { "Waiting for messages..." }
                }
            }
        }
    }
}
