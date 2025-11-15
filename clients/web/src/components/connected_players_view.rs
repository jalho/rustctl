use dioxus::prelude::*;

#[component]
pub fn ConnectedPlayersView(state: crate::state::State) -> Element {
    let players = &state.snapshot.ingame_state.players;
    let player_count = players.len();
    let player_suffix = if player_count != 1 { "s" } else { "" };

    rsx!(
        div { style: "background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 12px;",
            div { style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; flex-wrap: wrap; gap: 8px;",
                h2 { style: "color: #e6edf3; font-size: 15px; font-weight: 600; margin: 0;",
                    "Connected Players"
                }
                div { style: "color: #7d8590; font-size: 11px;", "{player_count} player{player_suffix}" }
            }
            {
                if players.is_empty() {
                    rsx! {
                        div { style: "display: flex; align-items: center; justify-content: center; padding: 30px 0; color: #7d8590; font-size: 13px;",
                            "No players connected"
                        }
                    }
                } else {
                    rsx! {
                        div { style: "border: 1px solid #21262d; border-radius: 6px; overflow-x: auto;",
                            // Header
                            div { style: "display: grid; grid-template-columns: 2fr 2fr 2fr 1fr 1fr; gap: 8px; padding: 10px 12px; background: #0d1117; border-bottom: 1px solid #21262d; font-size: 11px; font-weight: 600; color: #7d8590; min-width: 600px;",
                                div { "Display Name" }
                                div { "Steam ID" }
                                div { "Address" }
                                div { "Ping" }
                                div { "Health" }
                            }
                            // Player rows
                            for player in players {
                                div { style: "display: grid; grid-template-columns: 2fr 2fr 2fr 1fr 1fr; gap: 8px; padding: 10px 12px; border-bottom: 1px solid #21262d; font-size: 12px; color: #e6edf3; background: #161b22; min-width: 600px;",
                                    div { style: "font-weight: 500; color: #58a6ff; overflow: hidden; text-overflow: ellipsis;",
                                        "{player.display_name}"
                                    }
                                    div { style: "font-family: monospace; font-size: 11px; color: #7d8590; overflow: hidden; text-overflow: ellipsis;",
                                        "{player.steam_id}"
                                    }
                                    div { style: "font-family: monospace; font-size: 11px; overflow: hidden; text-overflow: ellipsis;",
                                        "{player.address}"
                                    }
                                    div { style: "font-family: monospace;", "{player.ping}" }
                                    div { style: "font-family: monospace;", "{player.health:.0}" }
                                }
                            }
                        }
                    }
                }
            }
            // Collapsible raw JSON
            details { style: "margin-top: 12px;",
                summary { style: "color: #58a6ff; font-size: 12px; cursor: pointer; padding: 6px 0; user-select: none;",
                    "View raw state (JSON)"
                }
                div { style: "background: #0d1117; border: 1px solid #21262d; border-radius: 6px; padding: 12px; overflow-x: auto; max-height: 300px; overflow-y: auto; margin-top: 8px;",
                    {
                        if let Ok(pretty) = serde_json::to_string_pretty(&state) {
                            rsx! {
                                pre { style: "margin: 0; font-family: 'SF Mono', Monaco, Inconsolata, 'Roboto Mono', Consolas, 'Courier New', monospace; font-size: 11px; line-height: 1.5; color: #e6edf3;",
                                    "{pretty}"
                                }
                            }
                        } else {
                            rsx! {
                                div { style: "color: #f85149; font-size: 13px;", "Failed to render snapshot" }
                            }
                        }
                    }
                }
            }
        }
    )
}
