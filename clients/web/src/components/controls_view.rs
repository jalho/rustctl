use dioxus::prelude::*;

#[component]
pub fn ControlsView(state: crate::state::State, app_tx: Signal<Option<async_channel::Sender<String>>>) -> Element {
    let command: Option<String> = match state.snapshot.game_server_state {
        rustctl_common::snapshot::GameServerStateExposed::Init => None,
        rustctl_common::snapshot::GameServerStateExposed::InstallingUpdates => None,
        rustctl_common::snapshot::GameServerStateExposed::InstalledAndConfigured { game_meta: _ } => None,
        rustctl_common::snapshot::GameServerStateExposed::LaunchingGame { game_meta: _ } => None,
        rustctl_common::snapshot::GameServerStateExposed::GameRunningHealthy { game_meta: _ } => Some(
            serde_json::to_string(&rustctl_common::command::DownstreamClientMessage::ServerSaveAndClose)
                .expect("serializing a static command should succeed"),
        ),
        rustctl_common::snapshot::GameServerStateExposed::SavingAndClosingGame {} => None,
        rustctl_common::snapshot::GameServerStateExposed::GameClosedManually => Some(
            serde_json::to_string(&rustctl_common::command::DownstreamClientMessage::ServerInstallOrUpdateAndStart)
                .expect("serializing a static command should succeed"),
        ),
        rustctl_common::snapshot::GameServerStateExposed::GameTerminatedUnexpectedly => None,
    };

    rsx!(
        h2 { "ControlsView" }

        button {
            onclick: move |_| {
                if let Some(tx) = &*app_tx.read() && let Some(command) = command.clone() {
                    let _ = tx.try_send(command);
                }
            },
            match state.snapshot.game_server_state {
                rustctl_common::snapshot::GameServerStateExposed::Init
                | rustctl_common::snapshot::GameServerStateExposed::InstallingUpdates
                | rustctl_common::snapshot::GameServerStateExposed::InstalledAndConfigured {
                    game_meta: _,
                }
                | rustctl_common::snapshot::GameServerStateExposed::LaunchingGame {
                    game_meta: _,
                }
                | rustctl_common::snapshot::GameServerStateExposed::SavingAndClosingGame {} => {
                    "Do nothing"
                }
                rustctl_common::snapshot::GameServerStateExposed::GameRunningHealthy {
                    game_meta: _,
                } => "Close game",
                rustctl_common::snapshot::GameServerStateExposed::GameClosedManually
                | rustctl_common::snapshot::GameServerStateExposed::GameTerminatedUnexpectedly => {
                    "Install updates and start game"
                }
            }
        }
    )
}
