use dioxus::prelude::*;
use futures_util::{
    StreamExt,
    stream::{SplitSink, SplitStream},
};
use gloo_net::websocket::{Message, futures::WebSocket};
use rustctl_common::web_app::WEBSOCKET_CONNECT_URL_PATH;
use wasm_bindgen_futures::spawn_local;

static REMOTE_STATE_SNAPSHOT: GlobalSignal<Option<String>> =
    GlobalSignal::<Option<String>>::new(|| None);

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut poc_signal: Signal<u32> = use_signal::<u32>(|| 0);

    let interval = std::time::Duration::from_secs(1);
    use_effect(move || {
        spawn_local(async move {
            'connect_websocket: loop {
                {
                    let mut locked = poc_signal.write();
                    *locked = *locked + 1;
                }

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

                let (tx, rx) = ws.split();
                let _tx: SplitSink<WebSocket, Message> = tx;
                let mut rx: SplitStream<WebSocket> = rx;

                'recv_messages: while let Some(msg_result) = rx.next().await {
                    match msg_result {
                        Ok(Message::Text(serialized)) => {
                            REMOTE_STATE_SNAPSHOT.with_mut(|state| {
                                *state = Some(serialized);
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
            p { "POC signal: {poc_signal}" }
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
            let serialized = snapshot;
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
