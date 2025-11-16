use crate::{
    BG_PRIMARY, BG_SECONDARY, BORDER_PRIMARY, BORDER_SECONDARY, FONT_MONO, FONT_SIZE_SM, FONT_SIZE_XL, FONT_SIZE_XS,
    RADIUS_MD, RADIUS_SM, SPACING_MD, SPACING_SM, TEXT_ERROR, TEXT_PRIMARY, TEXT_SECONDARY,
};
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

    let env_time = state.snapshot.ingame_state.env_time.0;
    let hours = env_time.floor() as i32;
    let minutes = ((env_time - env_time.floor()) * 60.0).round() as i32;
    let time_display = format!("{:02}:{:02}", hours, minutes);

    rsx! {
        div { style: "background: {BG_SECONDARY}; border: 1px solid {BORDER_PRIMARY}; border-radius: {RADIUS_MD}; padding: {SPACING_MD};",
            div { style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 10px; flex-wrap: wrap; gap: {SPACING_SM};",
                h2 { style: "color: {TEXT_PRIMARY}; font-size: {FONT_SIZE_XL}; font-weight: 600; margin: 0;",
                    "World Map"
                }
                div { style: "display: flex; align-items: center; gap: {SPACING_MD}; flex-wrap: wrap;",
                    div { style: "color: {TEXT_SECONDARY}; font-size: {FONT_SIZE_SM};",
                        "{player_count} player{player_suffix} online"
                    }
                    div { style: "color: {TEXT_SECONDARY}; font-size: {FONT_SIZE_SM};",
                        "Time: "
                        span { style: "color: {TEXT_PRIMARY}; font-family: {FONT_MONO};",
                            "{time_display}"
                        }
                    }
                }
            }
            div { style: "position: relative; width: 100%; aspect-ratio: 1; background: {BG_PRIMARY}; border: 1px solid {BORDER_SECONDARY}; border-radius: {RADIUS_MD}; overflow: hidden;",
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

    let marker_style =
        format!("position: absolute; left: {left}%; top: {top}%; transform: translate(-50%, -50%); z-index: 10;");

    rsx! {
        div { style: "{marker_style}", title: "{display_name}",
            div { style: "width: 12px; height: 12px; background: {TEXT_ERROR}; border: 2px solid {BG_PRIMARY}; border-radius: 50%; box-shadow: 0 0 0 2px rgba(248, 81, 73, 0.4); cursor: pointer;" }
            div { style: "position: absolute; top: 16px; left: 50%; transform: translateX(-50%); white-space: nowrap; background: rgba(22, 27, 34, 0.95); color: {TEXT_PRIMARY}; padding: 3px 6px; border-radius: {RADIUS_SM}; font-size: {FONT_SIZE_XS}; font-weight: 500; border: 1px solid {BORDER_PRIMARY}; pointer-events: none;",
                "{display_name}"
            }
        }
    }
}
