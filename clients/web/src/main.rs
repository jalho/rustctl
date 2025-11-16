mod components;
mod state;
mod websocket;

use components::{ConnectedPlayersView, ControlsView, MapView, PlayerResourcesView};
use dioxus::prelude::*;
use web_sys::wasm_bindgen::JsCast;
use web_sys::{HtmlBodyElement, window};

const BG_PRIMARY: &str = "#0d1117";
const BG_SECONDARY: &str = "#161b22";
const BG_TERTIARY: &str = "#21262d";

const BORDER_PRIMARY: &str = "#30363d";
const BORDER_SECONDARY: &str = "#21262d";

const TEXT_PRIMARY: &str = "#e6edf3";
const TEXT_SECONDARY: &str = "#7d8590";
const TEXT_ACCENT: &str = "#58a6ff";
const TEXT_ERROR: &str = "#f85149";

const STATUS_RUNNING: &str = "#238636";
const STATUS_INFO: &str = "#1f6feb";

const FONT_FAMILY: &str = "-apple-system,BlinkMacSystemFont,Segoe UI,Noto Sans,Helvetica,Arial,sans-serif";
const FONT_MONO: &str = "'SF Mono', Monaco, Inconsolata, 'Roboto Mono', Consolas, 'Courier New', monospace";

const SPACING_SM: &str = "8px";
const SPACING_MD: &str = "12px";
const SPACING_LG: &str = "16px";
const SPACING_XL: &str = "24px";

const SPACING_MOBILE: &str = "12px";

const RADIUS_SM: &str = "3px";
const RADIUS_MD: &str = "6px";
const RADIUS_LG: &str = "20px";

const FONT_SIZE_XS: &str = "10px";
const FONT_SIZE_SM: &str = "11px";
const FONT_SIZE_MD: &str = "12px";
const FONT_SIZE_BASE: &str = "13px";
const FONT_SIZE_LG: &str = "14px";
const FONT_SIZE_XL: &str = "15px";

fn main() {
    dioxus::launch(App);
}

#[component]
fn CardHeader(title: String, subtitle: Option<String>) -> Element {
    rsx! {
        div { style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: {SPACING_MD}; flex-wrap: wrap; gap: {SPACING_SM};",
            h2 { style: "color: {TEXT_PRIMARY}; font-size: {FONT_SIZE_XL}; font-weight: 600; margin: 0;",
                "{title}"
            }
            if let Some(sub) = subtitle {
                div { style: "color: {TEXT_SECONDARY}; font-size: {FONT_SIZE_SM};",
                    "{sub}"
                }
            }
        }
    }
}

#[component]
fn StatusBadge(label: String, color: String) -> Element {
    rsx! {
        div { style: "display: inline-flex; align-items: center; gap: 6px; background: {color}; color: #ffffff; padding: 3px 10px; border-radius: {RADIUS_LG}; font-size: {FONT_SIZE_SM}; font-weight: 500;",
            span { "●" }
            span { "{label}" }
        }
    }
}

#[component]
fn EmptyState(message: String) -> Element {
    rsx! {
        div { style: "display: flex; align-items: center; justify-content: center; padding: 30px 0; color: {TEXT_SECONDARY}; font-size: {FONT_SIZE_BASE};",
            "{message}"
        }
    }
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
                style.set_property("background-color", BG_PRIMARY).ok();
                style.set_property("color", TEXT_PRIMARY).ok();
                style.set_property("margin", "0").ok();
                style.set_property("padding", "0").ok();
                style.set_property("font-family", FONT_FAMILY).ok();
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
                div { style: "display: flex; justify-content: center; align-items: center; min-height: 100vh; background: {BG_PRIMARY};",
                    div { style: "background: {BG_SECONDARY}; border: 1px solid {BORDER_PRIMARY}; border-radius: {RADIUS_MD}; padding: {SPACING_XL}; text-align: center;",
                        p { style: "color: {TEXT_SECONDARY}; font-size: {FONT_SIZE_LG}; margin: 0;",
                            "Not connected to server"
                        }
                    }
                }
            };
        }
    };

    rsx! {
        div { style: "background: {BG_PRIMARY}; min-height: 100vh; padding: {SPACING_MOBILE};",
            div { style: "max-width: 1280px; margin: 0 auto;",
                div { style: "display: flex; flex-direction: column; gap: {SPACING_LG};",
                    MapView { state: state.clone() }
                    PlayerResourcesView { state: state.clone() }
                    ConnectedPlayersView { state: state.clone() }
                    ControlsView { state: state.clone(), app_tx }
                }
            }
        }
    }
}
