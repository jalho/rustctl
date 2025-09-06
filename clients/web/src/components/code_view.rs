use dioxus::prelude::*;

#[component]
pub fn CodeView(state: crate::state::State) -> Element {
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
