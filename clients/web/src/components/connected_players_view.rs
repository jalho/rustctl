use crate::{
    BG_PRIMARY, BG_SECONDARY, BORDER_PRIMARY, BORDER_SECONDARY, FONT_MONO, FONT_SIZE_BASE, FONT_SIZE_MD, FONT_SIZE_SM,
    FONT_SIZE_XL, RADIUS_MD, SPACING_MD, SPACING_SM, TEXT_ACCENT, TEXT_ERROR, TEXT_PRIMARY, TEXT_SECONDARY,
};
use dioxus::prelude::*;

#[component]
pub fn ConnectedPlayersView(state: crate::state::State) -> Element {
    let players = &state.snapshot.ingame_state.players;
    let player_count = players.len();
    let player_suffix = if player_count != 1 { "s" } else { "" };

    rsx!(
        div { style: "background: {BG_SECONDARY}; border: 1px solid {BORDER_PRIMARY}; border-radius: {RADIUS_MD}; padding: {SPACING_MD};",
            div { style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: {SPACING_MD}; flex-wrap: wrap; gap: {SPACING_SM};",
                h2 { style: "color: {TEXT_PRIMARY}; font-size: {FONT_SIZE_XL}; font-weight: 600; margin: 0;",
                    "Connected Players"
                }
                div { style: "color: {TEXT_SECONDARY}; font-size: {FONT_SIZE_SM};",
                    "{player_count} player{player_suffix}"
                }
            }
            {
                if players.is_empty() {
                    rsx! {
                        div { style: "display: flex; align-items: center; justify-content: center; padding: 30px 0; color: {TEXT_SECONDARY}; font-size: {FONT_SIZE_BASE};",
                            "No players connected"
                        }
                    }
                } else {
                    rsx! {
                        div { style: "display: flex; flex-direction: column; gap: {SPACING_MD};",
                            for player in players {
                                {
                                    let connected_time = format_connected_time(player.connected_seconds);
                                    rsx! {
                                        div { style: "background: {BG_PRIMARY}; border: 1px solid {BORDER_SECONDARY}; border-radius: {RADIUS_MD}; padding: {SPACING_MD};",
                                            div { style: "display: grid; grid-template-columns: auto 1fr; gap: {SPACING_SM} {SPACING_MD}; line-height: 1.8;",
                                                div { style: "color: {TEXT_SECONDARY}; font-size: {FONT_SIZE_BASE}; padding-top: 1px;",
                                                    "Display Name:"
                                                }
                                                div { style: "color: {TEXT_ACCENT}; font-weight: 500; font-size: {FONT_SIZE_BASE}; padding-top: 1px;",
                                                    "{player.display_name}"
                                                }
                                                div { style: "color: {TEXT_SECONDARY}; font-size: {FONT_SIZE_BASE}; padding-top: 1px;",
                                                    "Steam ID:"
                                                }
                                                div { style: "color: {TEXT_PRIMARY}; font-family: {FONT_MONO}; font-size: {FONT_SIZE_BASE};",
                                                    "{player.steam_id}"
                                                }
                                                div { style: "color: {TEXT_SECONDARY}; font-size: {FONT_SIZE_BASE}; padding-top: 1px;",
                                                    "Health:"
                                                }
                                                div { style: "color: {TEXT_PRIMARY}; font-family: {FONT_MONO}; font-size: {FONT_SIZE_BASE};",
                                                    "{player.health:.0}"
                                                }
                                                div { style: "color: {TEXT_SECONDARY}; font-size: {FONT_SIZE_BASE}; padding-top: 1px;",
                                                    "Connected:"
                                                }
                                                div { style: "color: {TEXT_PRIMARY}; font-family: {FONT_MONO}; font-size: {FONT_SIZE_BASE};",
                                                    "{connected_time}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            details { style: "margin-top: {SPACING_MD};",
                summary { style: "color: {TEXT_ACCENT}; font-size: {FONT_SIZE_MD}; cursor: pointer; padding: 6px 0; user-select: none;",
                    "View raw state (JSON)"
                }
                div { style: "background: {BG_PRIMARY}; border: 1px solid {BORDER_SECONDARY}; border-radius: {RADIUS_MD}; padding: {SPACING_MD}; overflow-x: auto; max-height: 300px; overflow-y: auto; margin-top: {SPACING_SM};",
                    {
                        if let Ok(pretty) = serde_json::to_string_pretty(&state) {
                            rsx! {
                                pre { style: "margin: 0; font-family: {FONT_MONO}; font-size: {FONT_SIZE_SM}; line-height: 1.5; color: {TEXT_PRIMARY};",
                                    "{pretty}"
                                }
                            }
                        } else {
                            rsx! {
                                div { style: "color: {TEXT_ERROR}; font-size: {FONT_SIZE_BASE};", "Failed to render snapshot" }
                            }
                        }
                    }
                }
            }
        }
    )
}

fn format_connected_time(seconds: i32) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        let minutes = seconds / 60;
        let secs = seconds % 60;
        format!("{}m {}s", minutes, secs)
    } else {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        format!("{}h {}m", hours, minutes)
    }
}
