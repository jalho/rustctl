mod components;
mod config;
mod state;
mod websocket;

use components::{AggregatedView, CodeView, MapView};
use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_effect(move || {
        websocket::start_websocket_connection();
    });

    let state = state::LATEST_SNAPSHOT.read();
    let state: Option<state::State> = state.clone();

    let state: state::State = match state {
        Some(n) => n,
        None => {
            return rsx! {
                p { "Not connected" }
            };
        }
    };

    rsx! {
        div {
            h1 { "WebSocket JSON Viewer" }
            CodeView { state: state.clone() }
            h2 { "Game World Map" }
            MapView {
                state: state.clone(),
                backend_url: config::BACKEND_URL
            }
            h2 { "Aggregated Resources" }
            AggregatedView { state: state.clone() }
        }
    }
}
