mod components;
mod config;
mod state;
mod websocket;

use components::{AggregatedView, CodeView, MapView};
use dioxus::prelude::*;
use web_sys::wasm_bindgen::JsCast;

fn main() {
    dioxus::launch(App);
}

use web_sys::{HtmlBodyElement, window};

#[component]
fn App() -> Element {
    use_effect(move || {
        websocket::start_websocket_connection();

        if let Some(document) = window().and_then(|w| w.document())
            && let Some(body) = document.body() {
                let body: HtmlBodyElement = body.dyn_into().unwrap();
                let style = body.style();
                style.set_property("background-color", "#0B3B4A").ok();
                style.set_property("color", "white").ok();
                style.set_property("margin", "1rem").ok();
                style.set_property("font-family", "sans-serif").ok();
                style.set_property("min-height", "100vh").ok();
            }
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
            h2 { "Game World Map" }
            MapView { state: state.clone(), backend_url: config::BACKEND_URL }

            h2 { "Aggregated Resources" }
            AggregatedView { state: state.clone() }

            h2 { "Debug" }
            CodeView { state: state.clone() }
        }
    }
}
