use dioxus::prelude::*;

#[component]
pub fn CodeView(state: crate::state::State) -> Element {
    let players = &state.snapshot.ingame_state.players;
    let player_count = players.len();
    let player_suffix = if player_count != 1 { "s" } else { "" };
    
    rsx!(
        div { style: "background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 16px;",
            div { style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 16px;",
                h2 { style: "color: #e6edf3; font-size: 16px; font-weight: 600; margin: 0;",
                    "Connected Players"
                }
                div { style: "color: #7d8590; font-size: 12px;", "{player_count} player{player_suffix}" }
            }
            {
                if players.is_empty() {
                    rsx! {
                        div { style: "display: flex; align-items: center; justify-content: center; padding: 40px 0; color: #7d8590; font-size: 14px;",
                            "No players connected"
                        }
                    }
                } else {
                    rsx! {
                        div { style: "border: 1px solid #21262d; border-radius: 6px; overflow: hidden;",
                            // Header
                            div { style: "display: grid; grid-template-columns: 2fr 3fr 2fr 1fr 1fr; gap: 16px; padding: 12px 16px; background: #0d1117; border-bottom: 1px solid #21262d; font-size: 12px; font-weight: 600; color: #7d8590;",
                                div { "Display Name" }
                                div { "Steam ID" }
                                div { "Address" }
                                div { "Ping" }
                                div { "Health" }
                            }
                            // Player rows
                            for player in players {
                                div { style: "display: grid; grid-template-columns: 2fr 3fr 2fr 1fr 1fr; gap: 16px; padding: 12px 16px; border-bottom: 1px solid #21262d; font-size: 13px; color: #e6edf3; background: #161b22;",
                                    div { style: "font-weight: 500; color: #58a6ff;", "{player.display_name}" }
                                    div { style: "font-family: monospace; font-size: 12px; color: #7d8590;",
                                        "{player.steam_id}"
                                    }
                                    div { style: "font-family: monospace; font-size: 12px;", "{player.address}" }
                                    div { style: "font-family: monospace;", "{player.ping}" }
                                    div { style: "font-family: monospace;", "{player.health:.0}" }
                                }
                            }
                        }
                    }
                }
            }
            // Collapsible raw JSON
            details { style: "margin-top: 16px;",
                summary { style: "color: #58a6ff; font-size: 13px; cursor: pointer; padding: 8px 0; user-select: none;",
                    "View raw state (JSON)"
                }
                div { style: "background: #0d1117; border: 1px solid #21262d; border-radius: 6px; padding: 16px; overflow-x: auto; max-height: 400px; overflow-y: auto; margin-top: 8px;",
                    {
                        if let Ok(pretty) = serde_json::to_string_pretty(&state) {
                            rsx! {
                                pre { style: "margin: 0; font-family: 'SF Mono', Monaco, Inconsolata, 'Roboto Mono', Consolas, 'Courier New', monospace; font-size: 12px; line-height: 1.6; color: #e6edf3;",
                                    "{pretty}"
                                }
                            }
                        } else {
                            rsx! {
                                div { style: "color: #f85149; font-size: 14px;", "Failed to render snapshot" }
                            }
                        }
                    }
                }
            }
        }
    )
}
