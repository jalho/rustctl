use dioxus::dioxus_core;
use dioxus::prelude::dioxus_elements;
use dioxus::prelude::dioxus_signals;

#[dioxus::prelude::component]
pub fn DebugViewer() -> dioxus::core::Element {
    let state: crate::state::GlobalState =
        dioxus::hooks::use_context::<crate::state::GlobalState>();

    let attempts = dioxus_signals::ReadableExt::read(&state.connection_attempts);

    dioxus::prelude::rsx! {
        div {
            p { "Connection Attempts: {attempts}" }
        }
    }
}
