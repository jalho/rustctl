mod components;
mod state;
mod websocket;

use components::{ConnectedPlayersView, ControlsView, MapView, PlayerResourcesView};
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
        let mut app_tx = app_tx;
        move || {
            let (tx, rx) = async_channel::unbounded::<String>();
            websocket::start_websocket_connection(rx);
            app_tx.set(Some(tx));

            if let Some(document) = window().and_then(|w| w.document())
                && let Some(body) = document.body()
            {
                let body: HtmlBodyElement = body.dyn_into().unwrap();
                let style = body.style();
                style.set_property("background-color", "#0d1117").ok();
                style.set_property("color", "#e6edf3").ok();
                style.set_property("margin", "0").ok();
                style.set_property("padding", "0").ok();
                style
                    .set_property(
                        "font-family",
                        "-apple-system,BlinkMacSystemFont,Segoe UI,Noto Sans,Helvetica,Arial,sans-serif",
                    )
                    .ok();
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
                div { style: "display: flex; justify-content: center; align-items: center; min-height: 100vh; background: #0d1117;",
                    div { style: "background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 24px; text-align: center;",
                        p { style: "color: #7d8590; font-size: 14px; margin: 0;",
                            "Not connected to server"
                        }
                    }
                }
            };
        }
    };

    rsx! {
        div { style: "background: #0d1117; min-height: 100vh; padding: 16px;",
            div { style: "max-width: 1280px; margin: 0 auto;",
                div { style: "margin-bottom: 16px;",
                    h1 { style: "color: #e6edf3; font-size: 24px; font-weight: 600; margin: 0 0 4px 0;",
                        "rustctl"
                    }
                    p { style: "color: #7d8590; font-size: 13px; margin: 0;",
                        "Rust game server management"
                    }
                }

                ControlsView { state: state.clone(), app_tx }

                div { style: "display: grid; grid-template-columns: 1fr; gap: 16px; margin-bottom: 16px;",
                    MapView { state: state.clone() }
                    PlayerResourcesView { state: state.clone() }
                    ConnectedPlayersView { state: state.clone() }
                }
            }
        }
    }
}
