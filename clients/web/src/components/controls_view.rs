use dioxus::prelude::*;

#[component]
pub fn ControlsView(state: crate::state::State, app_tx: Signal<Option<async_channel::Sender<String>>>) -> Element {
    let controls: Controls = match state.snapshot.game_server_state {
        rustctl_common::snapshot::GameServerStateExposed::Init => Controls::NotActionable {
            state_display: "Init".to_string(),
        },

        rustctl_common::snapshot::GameServerStateExposed::InstallingUpdates => Controls::NotActionable {
            state_display: "InstallingUpdates".to_string(),
        },

        rustctl_common::snapshot::GameServerStateExposed::InstalledAndConfigured { game_meta: _ } => {
            Controls::NotActionable {
                state_display: "InstalledAndConfigured".to_string(),
            }
        }

        rustctl_common::snapshot::GameServerStateExposed::LaunchingGame { game_meta: _ } => Controls::NotActionable {
            state_display: "LaunchingGame".to_string(),
        },

        rustctl_common::snapshot::GameServerStateExposed::GameRunningHealthy { game_meta: _ } => {
            let command_serialized =
                serde_json::to_string(&rustctl_common::command::DownstreamClientMessage::ServerSaveAndClose)
                    .expect("serializing static command should succeed");

            Controls::Actionable {
                command_display: command_serialized.clone(),
                command_serialized,
            }
        }

        rustctl_common::snapshot::GameServerStateExposed::SavingAndClosingGame {} => Controls::NotActionable {
            state_display: "SavingAndClosingGame".to_string(),
        },

        rustctl_common::snapshot::GameServerStateExposed::GameClosedManually => {
            let command_serialized =
                serde_json::to_string(&rustctl_common::command::DownstreamClientMessage::ServerInstallOrUpdateAndStart)
                    .expect("serializing static command should succeed");

            Controls::Actionable {
                command_display: command_serialized.clone(),
                command_serialized,
            }
        }

        rustctl_common::snapshot::GameServerStateExposed::GameTerminatedUnexpectedly => {
            let command_serialized =
                serde_json::to_string(&rustctl_common::command::DownstreamClientMessage::ServerInstallOrUpdateAndStart)
                    .expect("serializing static command should succeed");

            Controls::Actionable {
                command_display: command_serialized.clone(),
                command_serialized,
            }
        }
    };

    match controls {
        Controls::Actionable {
            command_serialized,
            command_display,
        } => {
            rsx!(
                h2 { "ControlsView" }

                button {
                    onclick: move |_| {
                        if let Some(tx) = &*app_tx.read() {
                            let _ = tx.try_send(command_serialized.clone());
                        }
                    },
                    {command_display}
                }
            )
        }

        Controls::NotActionable { state_display } => {
            rsx!(
                h2 { "ControlsView" }

                p {
                    {state_display}
                }
            )
        }
    }
}

enum Controls {
    Actionable {
        command_serialized: String,
        command_display: String,
    },

    NotActionable {
        state_display: String,
    },
}
