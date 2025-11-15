use dioxus::prelude::*;

#[component]
pub fn ControlsView(state: crate::state::State, app_tx: Signal<Option<async_channel::Sender<String>>>) -> Element {
    let controls: Controls = match state.snapshot.game_server_state {
        rustctl_common::snapshot::GameServerStateExposed::Init => Controls::NotActionable {
            state_display: "Initializing".to_string(),
            state_color: "#58a6ff".to_string(),
        },

        rustctl_common::snapshot::GameServerStateExposed::InstallingUpdates => Controls::NotActionable {
            state_display: "Installing updates".to_string(),
            state_color: "#58a6ff".to_string(),
        },

        rustctl_common::snapshot::GameServerStateExposed::InstalledAndConfigured { game_meta: _ } => {
            Controls::NotActionable {
                state_display: "Installed and configured".to_string(),
                state_color: "#58a6ff".to_string(),
            }
        }

        rustctl_common::snapshot::GameServerStateExposed::LaunchingGame { game_meta: _ } => Controls::NotActionable {
            state_display: "Launching game".to_string(),
            state_color: "#58a6ff".to_string(),
        },

        rustctl_common::snapshot::GameServerStateExposed::GameRunningHealthy { game_meta: _ } => {
            let command_serialized =
                serde_json::to_string(&rustctl_common::command::DownstreamClientMessage::ServerSaveAndClose)
                    .expect("serializing static command should succeed");

            Controls::Actionable {
                command_display: "Stop server".to_string(),
                command_serialized,
                state_display: "Running".to_string(),
            }
        }

        rustctl_common::snapshot::GameServerStateExposed::SavingAndClosingGame {} => Controls::NotActionable {
            state_display: "Saving and closing".to_string(),
            state_color: "#f85149".to_string(),
        },

        rustctl_common::snapshot::GameServerStateExposed::GameClosedManually => {
            let command_serialized =
                serde_json::to_string(&rustctl_common::command::DownstreamClientMessage::ServerInstallOrUpdateAndStart)
                    .expect("serializing static command should succeed");

            Controls::Actionable {
                command_display: "Start server".to_string(),
                command_serialized,
                state_display: "Stopped".to_string(),
            }
        }

        rustctl_common::snapshot::GameServerStateExposed::GameTerminatedUnexpectedly => {
            let command_serialized =
                serde_json::to_string(&rustctl_common::command::DownstreamClientMessage::ServerInstallOrUpdateAndStart)
                    .expect("serializing static command should succeed");

            Controls::Actionable {
                command_display: "Restart server".to_string(),
                command_serialized,
                state_display: "Terminated".to_string(),
            }
        }
    };

    match controls {
        Controls::Actionable {
            command_serialized,
            command_display,
            state_display,
        } => {
            rsx!(
                div { style: "background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 12px; margin-bottom: 16px;",
                    div { style: "display: flex; flex-direction: column; gap: 10px;",
                        div { style: "display: flex; align-items: center; gap: 10px; flex-wrap: wrap;",
                            h2 { style: "color: #e6edf3; font-size: 15px; font-weight: 600; margin: 0;",
                                "Server Control"
                            }
                            div { style: "display: inline-flex; align-items: center; gap: 6px; background: #238636; color: #ffffff; padding: 3px 10px; border-radius: 20px; font-size: 11px; font-weight: 500;",
                                span { "●" }
                                span { "{state_display}" }
                            }
                        }
                        button {
                            style: "background: #21262d; color: #c9d1d9; border: 1px solid #30363d; border-radius: 6px; padding: 8px 16px; font-size: 14px; font-weight: 500; cursor: pointer; width: 100%;",
                            onclick: move |_| {
                                if let Some(tx) = &*app_tx.read() {
                                    let _ = tx.try_send(command_serialized.clone());
                                }
                            },
                            {command_display}
                        }
                    }
                }
            )
        }

        Controls::NotActionable {
            state_display,
            state_color,
        } => {
            rsx!(
                div { style: "background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 12px; margin-bottom: 16px;",
                    div { style: "display: flex; align-items: center; gap: 10px; flex-wrap: wrap;",
                        h2 { style: "color: #e6edf3; font-size: 15px; font-weight: 600; margin: 0;",
                            "Server Control"
                        }
                        div { style: "display: inline-flex; align-items: center; gap: 6px; background: #1f6feb; color: #ffffff; padding: 3px 10px; border-radius: 20px; font-size: 11px; font-weight: 500;",
                            span { style: "color: {state_color};", "●" }
                            span { "{state_display}" }
                        }
                    }
                }
            )
        }
    }
}

enum Controls {
    Actionable {
        command_serialized: String,
        command_display: String,
        state_display: String,
    },

    NotActionable {
        state_display: String,
        state_color: String,
    },
}
