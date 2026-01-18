use dioxus::dioxus_core;
use dioxus::document;
use dioxus::prelude::asset;
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::manganis;

#[dioxus::prelude::component]
pub fn App() -> dioxus::core::Element {
    /*
     * Initialize a global state, and hook it to a WebSocket.
     */
    dioxus::hooks::use_context_provider(crate::state::GlobalState::init);
    dioxus::prelude::use_future(crate::state::GlobalState::connect_websocket);

    /*
     * Render stuff from the global state.
     */
    dioxus::prelude::rsx! {
        document::Link { rel: "stylesheet", href: asset!("/assets/main.css") }
        crate::layout::debug_viewer::DebugViewer {}
    }
}
