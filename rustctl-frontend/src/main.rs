use dioxus::prelude::*;
use futures_util::StreamExt;
use gloo_net::websocket::{Message, futures::WebSocket};
use wasm_bindgen_futures::spawn_local;

static GLOBAL_SIGNAL: GlobalSignal<std::string::String> =
    GlobalSignal::<std::string::String>::new(|| "Waiting for messages...".to_string());

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_effect(move || {
        spawn_local(async move {
            let ws = WebSocket::open("ws://localhost:8081/api/websocket").unwrap();
            let (_write, mut read) = ws.split();

            while let Some(Ok(Message::Text(payload))) = read.next().await {
                GLOBAL_SIGNAL.with_mut(|state| {
                    *state = payload;
                });
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

    rsx! {
        div {
            h2 { "Latest Message:" }
            p { "{value}" }
        }
    }
}
