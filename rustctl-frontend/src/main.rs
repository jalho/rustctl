use dioxus::prelude::*;
use futures::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use gloo_net::websocket::{Message, futures::WebSocket};
use rustctl_common::{snapshot::Snapshot, web_app::WEBSOCKET_CONNECT_URL_PATH};
use wasm_bindgen_futures::spawn_local;

static REMOTE_STATE_SNAPSHOT: GlobalSignal<Option<Snapshot>> =
    GlobalSignal::<Option<Snapshot>>::new(|| None);

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut sender: Signal<Option<futures::channel::mpsc::UnboundedSender<String>>> =
        use_signal(|| None);
    let interval = std::time::Duration::from_secs(1);

    use_effect(move || {
        spawn_local(async move {
            'connect_websocket: loop {
                let ws_result = WebSocket::open(&format!(
                    "ws://localhost:8081{path}",
                    path = WEBSOCKET_CONNECT_URL_PATH
                ));
                let ws: WebSocket = match ws_result {
                    Ok(ws) => ws,
                    Err(_) => {
                        gloo_timers::future::sleep(interval).await;
                        continue 'connect_websocket;
                    }
                };

                let (sock_tx, sock_rx) = ws.split();
                let mut sock_tx: SplitSink<WebSocket, Message> = sock_tx;
                let mut sock_rx: SplitStream<WebSocket> = sock_rx;

                let (tx, mut rx) = futures::channel::mpsc::unbounded::<String>();
                {
                    let mut locked = sender.write();
                    *locked = Some(tx);
                }

                let _coroutine_tx = spawn_local(async move {
                    while let Some(message) = futures::StreamExt::next(&mut rx).await {
                        if let Err(_) = sock_tx.send(Message::Text(message)).await {
                            break;
                        }
                    }
                });

                'recv_messages: while let Some(msg_result) = sock_rx.next().await {
                    match msg_result {
                        Ok(Message::Text(serialized)) => {
                            REMOTE_STATE_SNAPSHOT.with_mut(|state| {
                                let deserialized: Snapshot =
                                    serde_json::from_str(&serialized).unwrap();
                                *state = Some(deserialized);
                            });
                        }
                        Ok(_) => {}
                        Err(_) => break 'recv_messages,
                    }
                }

                {
                    let mut locked = sender.write();
                    *locked = None;
                }

                gloo_timers::future::sleep(interval).await;
            }
        });
    });

    rsx! {
        div {
            button {
                onclick: move |event| {
                    event.stop_propagation();
                    gloo_console::log!("Button clicked!");
                    let sender = {
                        let locked = sender.read();
                        locked.clone()
                    };
                    if let Some(sender) = sender {
                        if let Err(_) = sender.unbounded_send("Hello from WebSocket client!".to_string()) {
                            gloo_console::log!("Failed to send message - channel closed");
                        }
                    } else {
                        gloo_console::log!("WebSocket not connected");
                    }
                },
                "Send"
            }
            h1 { "WebSocket Message Viewer" }
            MessageView {}
        }
    }
}

#[component]
fn MessageView() -> Element {
    let value = REMOTE_STATE_SNAPSHOT.read();
    match *value {
        Some(ref snapshot) => {
            let serialized = serde_json::to_string_pretty(snapshot).unwrap();
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
