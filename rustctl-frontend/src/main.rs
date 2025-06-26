use dioxus::prelude::*;
use futures::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use gloo_net::websocket::{Message, futures::WebSocket};
use rustctl_common::{command::Command, snapshot::Snapshot, web_app::WEBSOCKET_CONNECT_URL_PATH};
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

                spawn_local(async move {
                    while let Some(message) = futures::StreamExt::next(&mut rx).await {
                        if (sock_tx.send(Message::Text(message)).await).is_err() {
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

                /*
                 * Reset state when disconnected.
                 */
                {
                    let mut locked_tx = sender.write();
                    *locked_tx = None;

                    let mut locked_remote_state = REMOTE_STATE_SNAPSHOT.write();
                    *locked_remote_state = None;
                }

                gloo_timers::future::sleep(interval).await;
            }
        });
    });

    let transition_available: Option<Command> = match *REMOTE_STATE_SNAPSHOT.read() {
        Some(ref state) => {
            let state: &Snapshot = state;
            match state.game_server_state {
                rustctl_common::snapshot::GameServerStateExposed::Wiping(_)
                | rustctl_common::snapshot::GameServerStateExposed::Updating(_)
                | rustctl_common::snapshot::GameServerStateExposed::Stopping(_)
                | rustctl_common::snapshot::GameServerStateExposed::Launching(_) => None,
                rustctl_common::snapshot::GameServerStateExposed::NotRunning(_) => {
                    Some(Command::TransitionFromNotRunning)
                }
                rustctl_common::snapshot::GameServerStateExposed::RunningHealthy(_) => {
                    Some(Command::TransitionFromRunningHealthy)
                }
            }
        }
        None => None,
    };

    rsx! {
        div {
            button {
                disabled: match transition_available {
                    Some(_) => false,
                    None => true,
                },
                onclick: move |event| {
                    event.stop_propagation();
                    let command: &Command = match transition_available {
                        Some(ref n) => n,
                        None => return,
                    };
                    let serialized: String = serde_json::to_string(&command).unwrap();
                    let sender = {
                        let locked = sender.read();
                        locked.clone()
                    };
                    if let Some(sender) = sender {
                        if sender
                            .unbounded_send(serialized).is_err()
                        {
                            gloo_console::log!("Failed to send message - channel closed");
                        }
                    } else {
                        gloo_console::log!("WebSocket not connected");
                    }
                },
                match transition_available {
                    Some(ref n) => format!("{n:?}"),
                    None => "N/A".to_string(),
                }
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
