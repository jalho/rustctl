use dioxus::prelude::*;

#[component]
pub fn ControlsView(state: crate::state::State, app_tx: Signal<Option<async_channel::Sender<String>>>) -> Element {
    rsx!(
        h2 { "ControlsView" }

        button {
            onclick: move |_| {
                if let Some(tx) = &*app_tx.read() {
                    let _ = tx.try_send("some command here".to_string());
                }
            },
            "Do something"
        }
    )
}
