mod components;
mod config;
mod state;
mod websocket;

use components::{AggregatedView, CodeView, MapView, ControlsView};
use dioxus::prelude::*;
use web_sys::wasm_bindgen::JsCast;
use web_sys::{HtmlBodyElement, window};

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let app_tx = use_signal(|| None::<async_channel::Sender<String>>);

    use_effect({
        let mut app_tx = app_tx.clone();
        move || {
            let (tx, rx) = async_channel::unbounded::<String>();
            websocket::start_websocket_connection(rx);
            app_tx.set(Some(tx));

            if let Some(document) = window().and_then(|w| w.document())
                && let Some(body) = document.body()
            {
                let body: HtmlBodyElement = body.dyn_into().unwrap();
                let style = body.style();
                style.set_property("background-color", "#0B3B4A").ok();
                style.set_property("color", "white").ok();
                style.set_property("margin", "1rem").ok();
                style.set_property("font-family", "sans-serif").ok();
                style.set_property("min-height", "100vh").ok();
            }
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
            ControlsView { state: state.clone(), app_tx }

            MapView { state: state.clone(), backend_url: config::BACKEND_URL }

            AggregatedView { state: state.clone() }

            CodeView { state: state.clone() }
        }
    }
}
