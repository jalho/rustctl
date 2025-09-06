use dioxus::prelude::*;

const WORLD_MAP_RENDER_MARGIN: f64 = 1000.0;

#[component]
pub fn MapView(state: crate::state::State, backend_url: &'static str) -> Element {
    let map_width = 800.0;
    let map_height = 800.0;
    let world_size = state.snapshot.game_world_size + WORLD_MAP_RENDER_MARGIN;
    let world_half = world_size / 2.0;

    rsx! {
        div { style: "position: relative; width: {map_width}px; height: {map_height}px;",
            img {
                src: format!("{}{}", backend_url, rustctl_common::web_app::MAP_URL_PATH),
                alt: "Current game world map",
                style: "width: 100%; height: 100%; display: block;",
            }
            for player in &state.snapshot.ingame_state.players_pos {
                {
                    let (x, _y, z) = player.position;
                    let left: f64 = (x + world_half) / world_size * map_width;
                    let top: f64 = (world_half - z) / world_size * map_height;
                    let player_style = make_player_style(left, top);
                    rsx! {
                        div { style: "{player_style}", title: "{player.display_name}" }
                    }
                }
            }
        }
    }
}

fn make_player_style(left: f64, top: f64) -> String {
    format!(
        "position: absolute;
left: {left}px;
top: {top}px;
width: 10px;
height: 10px;
background: red;
border-radius: 50%;
transform: translate(-50%, -50%);",
    )
}
