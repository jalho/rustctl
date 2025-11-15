use dioxus::prelude::*;

const WORLD_MAP_RENDER_MARGIN: f64 = 1000.0;

#[component]
pub fn MapView(state: crate::state::State) -> Element {
    let map_width = 600.0;
    let map_height = 600.0;
    let world_size = state.snapshot.game_world_size + WORLD_MAP_RENDER_MARGIN;
    let world_half = world_size / 2.0;

    let map_url = if cfg!(debug_assertions) {
        format!(
            "http://rustctl.internal:8080{url_path}",
            url_path = rustctl_common::web_app::MAP_URL_PATH,
        )
    } else {
        rustctl_common::web_app::MAP_URL_PATH.to_string()
    };

    let player_count = state.snapshot.ingame_state.players_pos.len();
    let player_suffix = if player_count != 1 { "s" } else { "" };

    rsx! {
        div { style: "background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 16px; height: 100%;",
            div { style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px;",
                h2 { style: "color: #e6edf3; font-size: 16px; font-weight: 600; margin: 0;",
                    "World Map"
                }
                div { style: "color: #7d8590; font-size: 12px;",
                    "{player_count} player{player_suffix} online"
                }
            }
            div { style: "position: relative; width: 100%; aspect-ratio: 1; background: #0d1117; border: 1px solid #21262d; border-radius: 6px; overflow: hidden;",
                img {
                    src: "{map_url}",
                    alt: "Current game world map",
                    style: "width: 100%; height: 100%; display: block; object-fit: contain;",
                }
                for player in &state.snapshot.ingame_state.players_pos {
                    PlayerMarker {
                        position: player.position,
                        display_name: player.display_name.clone(),
                        map_width,
                        map_height,
                        world_size,
                        world_half,
                    }
                }
            }
        }
    }
}

#[component]
fn PlayerMarker(
    position: (f64, f64, f64),
    display_name: String,
    map_width: f64,
    map_height: f64,
    world_size: f64,
    world_half: f64,
) -> Element {
    let (x, _y, z) = position;
    let left: f64 = (x + world_half) / world_size * 100.0;
    let top: f64 = (world_half - z) / world_size * 100.0;

    let marker_style = format!(
        "position: absolute; left: {left}%; top: {top}%; transform: translate(-50%, -50%); z-index: 10;"
    );

    rsx! {
        div { style: "{marker_style}", title: "{display_name}",
            div { style: "width: 12px; height: 12px; background: #f85149; border: 2px solid #0d1117; border-radius: 50%; box-shadow: 0 0 0 2px rgba(248, 81, 73, 0.4); cursor: pointer;" }
            div { style: "position: absolute; top: 16px; left: 50%; transform: translateX(-50%); white-space: nowrap; background: rgba(22, 27, 34, 0.95); color: #e6edf3; padding: 4px 8px; border-radius: 4px; font-size: 11px; font-weight: 500; border: 1px solid #30363d; pointer-events: none;",
                "{display_name}"
            }
        }
    }
}
