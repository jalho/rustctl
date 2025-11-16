use crate::{
    BG_PRIMARY, BG_SECONDARY, BG_TERTIARY, BORDER_PRIMARY, BORDER_SECONDARY, FONT_MONO, FONT_SIZE_BASE, FONT_SIZE_LG,
    FONT_SIZE_SM, FONT_SIZE_XL, RADIUS_LG, RADIUS_MD, SPACING_LG, SPACING_MD, SPACING_SM, STATUS_INFO, STATUS_RUNNING,
    TEXT_ACCENT, TEXT_ERROR, TEXT_PRIMARY, TEXT_SECONDARY,
};
use dioxus::prelude::*;

#[component]
pub fn ControlsView(state: crate::state::State, app_tx: Signal<Option<async_channel::Sender<String>>>) -> Element {
    let controls: Controls = match state.snapshot.game_server_state {
        rustctl_common::snapshot::GameServerStateExposed::Init => Controls::NotActionable {
            state_display: "Initializing".to_string(),
            state_color: TEXT_ACCENT.to_string(),
            game_meta: None,
        },

        rustctl_common::snapshot::GameServerStateExposed::InstallingUpdates => Controls::NotActionable {
            state_display: "Installing updates".to_string(),
            state_color: TEXT_ACCENT.to_string(),
            game_meta: None,
        },

        rustctl_common::snapshot::GameServerStateExposed::InstalledAndConfigured { game_meta } => {
            Controls::NotActionable {
                state_display: "Installed and configured".to_string(),
                state_color: TEXT_ACCENT.to_string(),
                game_meta: Some(game_meta),
            }
        }

        rustctl_common::snapshot::GameServerStateExposed::LaunchingGame { game_meta } => Controls::NotActionable {
            state_display: "Launching game".to_string(),
            state_color: TEXT_ACCENT.to_string(),
            game_meta: Some(game_meta),
        },

        rustctl_common::snapshot::GameServerStateExposed::GameRunningHealthy { game_meta } => {
            let command_serialized =
                serde_json::to_string(&rustctl_common::command::DownstreamClientMessage::ServerSaveAndClose)
                    .expect("serializing static command should succeed");

            Controls::Actionable {
                command_display: "Stop server".to_string(),
                command_serialized,
                state_display: "Running".to_string(),
                game_meta: Some(game_meta),
            }
        }

        rustctl_common::snapshot::GameServerStateExposed::SavingAndClosingGame {} => Controls::NotActionable {
            state_display: "Saving and closing".to_string(),
            state_color: TEXT_ERROR.to_string(),
            game_meta: None,
        },

        rustctl_common::snapshot::GameServerStateExposed::GameClosedManually => {
            let command_serialized =
                serde_json::to_string(&rustctl_common::command::DownstreamClientMessage::ServerInstallOrUpdateAndStart)
                    .expect("serializing static command should succeed");

            Controls::Actionable {
                command_display: "Start server".to_string(),
                command_serialized,
                state_display: "Stopped".to_string(),
                game_meta: None,
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
                game_meta: None,
            }
        }
    };

    let memory_kib = state.snapshot.memory_used_kibibytes.into_u64();
    let memory_gib = memory_kib as f64 / 1_048_576.0;
    let cpus = &state.snapshot.cpus_utilization_percentage;

    match controls {
        Controls::Actionable {
            command_serialized,
            command_display,
            state_display,
            game_meta,
        } => {
            rsx!(
                div { style: "background: {BG_SECONDARY}; border: 1px solid {BORDER_PRIMARY}; border-radius: {RADIUS_MD}; padding: {SPACING_MD}; margin-bottom: {SPACING_LG};",
                    div { style: "display: flex; flex-direction: column; gap: {SPACING_MD};",
                        div { style: "display: flex; align-items: center; gap: 10px; flex-wrap: wrap;",
                            h2 { style: "color: {TEXT_PRIMARY}; font-size: {FONT_SIZE_XL}; font-weight: 600; margin: 0;",
                                "Server Control"
                            }
                            div { style: "display: inline-flex; align-items: center; gap: 6px; background: {STATUS_RUNNING}; color: #ffffff; padding: 3px 10px; border-radius: {RADIUS_LG}; font-size: {FONT_SIZE_SM}; font-weight: 500;",
                                span { "●" }
                                span { "{state_display}" }
                            }
                        }
                        if let Some(meta) = game_meta {
                            div { style: "background: {BG_PRIMARY}; border: 1px solid {BORDER_SECONDARY}; border-radius: {RADIUS_MD}; padding: {SPACING_MD};",
                                div { style: "display: grid; grid-template-columns: auto 1fr; gap: {SPACING_SM} {SPACING_MD}; line-height: 1.8;",
                                    div { style: "color: {TEXT_SECONDARY}; font-size: {FONT_SIZE_BASE}; padding-top: 1px;",
                                        "Build ID:"
                                    }
                                    div { style: "color: {TEXT_PRIMARY}; font-family: {FONT_MONO}; font-size: {FONT_SIZE_BASE};",
                                        "{meta.buildid}"
                                    }
                                    div { style: "color: {TEXT_SECONDARY}; font-size: {FONT_SIZE_BASE}; padding-top: 1px;",
                                        "Memory:"
                                    }
                                    div { style: "color: {TEXT_PRIMARY}; font-family: {FONT_MONO}; font-size: {FONT_SIZE_BASE};",
                                        "{memory_kib} KiB ({memory_gib:.2} GiB)"
                                    }
                                    for (idx , cpu) in cpus.iter().enumerate() {
                                        div { style: "color: {TEXT_SECONDARY}; font-size: {FONT_SIZE_BASE}; padding-top: 1px;",
                                            "CPU {idx}:"
                                        }
                                        div { style: "color: {TEXT_PRIMARY}; font-family: {FONT_MONO}; font-size: {FONT_SIZE_BASE};",
                                            "{cpu.as_percentage():.1}%"
                                        }
                                    }
                                }
                            }
                        }
                        button {
                            style: "background: {BG_TERTIARY}; color: {TEXT_PRIMARY}; border: 1px solid {BORDER_PRIMARY}; border-radius: {RADIUS_MD}; padding: {SPACING_SM} {SPACING_LG}; font-size: {FONT_SIZE_LG}; font-weight: 500; cursor: pointer; width: 100%;",
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
            game_meta,
        } => {
            rsx!(
                div { style: "background: {BG_SECONDARY}; border: 1px solid {BORDER_PRIMARY}; border-radius: {RADIUS_MD}; padding: {SPACING_MD}; margin-bottom: {SPACING_LG};",
                    div { style: "display: flex; flex-direction: column; gap: {SPACING_MD};",
                        div { style: "display: flex; align-items: center; gap: 10px; flex-wrap: wrap;",
                            h2 { style: "color: {TEXT_PRIMARY}; font-size: {FONT_SIZE_XL}; font-weight: 600; margin: 0;",
                                "Server Control"
                            }
                            div { style: "display: inline-flex; align-items: center; gap: 6px; background: {STATUS_INFO}; color: #ffffff; padding: 3px 10px; border-radius: {RADIUS_LG}; font-size: {FONT_SIZE_SM}; font-weight: 500;",
                                span { style: "color: {state_color};", "●" }
                                span { "{state_display}" }
                            }
                        }
                        if let Some(meta) = game_meta {
                            div { style: "background: {BG_PRIMARY}; border: 1px solid {BORDER_SECONDARY}; border-radius: {RADIUS_MD}; padding: {SPACING_MD};",
                                div { style: "display: grid; grid-template-columns: auto 1fr; gap: {SPACING_SM} {SPACING_MD}; font-size: {FONT_SIZE_BASE};",
                                    div { style: "color: {TEXT_SECONDARY};", "Build ID:" }
                                    div { style: "color: {TEXT_PRIMARY}; font-family: {FONT_MONO};",
                                        "{meta.buildid}"
                                    }
                                    div { style: "color: {TEXT_SECONDARY};", "Memory:" }
                                    div { style: "color: {TEXT_PRIMARY}; font-family: {FONT_MONO}; font-size: {FONT_SIZE_SM};",
                                        "{memory_kib} KiB ({memory_gib:.2} GiB)"
                                    }
                                    for (idx , cpu) in cpus.iter().enumerate() {
                                        div { style: "color: {TEXT_SECONDARY};", "CPU {idx}:" }
                                        div { style: "color: {TEXT_PRIMARY}; font-family: {FONT_MONO};",
                                            "{cpu.as_percentage():.1}%"
                                        }
                                    }
                                }
                            }
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
        game_meta: Option<rustctl_common::snapshot::GameServerMetaExposed>,
    },

    NotActionable {
        state_display: String,
        state_color: String,
        game_meta: Option<rustctl_common::snapshot::GameServerMetaExposed>,
    },
}
