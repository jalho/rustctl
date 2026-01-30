use dioxus::dioxus_core;
use dioxus::prelude::dioxus_elements;
use dioxus::prelude::dioxus_signals;

#[dioxus::prelude::component]
pub fn DebugViewer() -> dioxus::core::Element {
    let state = dioxus::hooks::use_context::<crate::state::GlobalState>();

    let last_msg = dioxus::signals::ReadableExt::read(&state.last_message);

    dioxus::prelude::rsx! {
        div {
            p { "{last_msg:?}" }
        }
    }
}
