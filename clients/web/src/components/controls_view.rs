use dioxus::prelude::*;

#[component]
pub fn ControlsView(state: crate::state::State, app_tx: Signal<Option<async_channel::Sender<String>>>) -> Element {
    rsx!(
        h2 { "ControlsView" }

        button {
            onclick: move |_| {
                if let Some(tx) = &*app_tx.read() {
                    let command = rustctl_common::command::DownstreamClientMessage::ServerSaveAndClose;
                    let serialized: String = serde_json::to_string(&command)
                        .expect("serializing a static command should succeed");
                    let _ = tx.try_send(serialized);
                }
            },
            "Do something"
        }
    )
}
