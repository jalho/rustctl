use crate::State;
use dioxus::prelude::*;

#[component]
pub fn CodeView(state: Option<State>) -> Element {
    match state {
        Some(state) => {
            if let Ok(pretty) = serde_json::to_string_pretty(&state) {
                rsx!(
                    pre { "{pretty}" }
                )
            } else {
                rsx!(
                    p { "Failed to render snapshot" }
                )
            }
        }
        None => rsx!(
            p { "Waiting for messages..." }
        ),
    }
}
