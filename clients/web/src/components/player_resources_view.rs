use crate::{
    BG_PRIMARY, BG_SECONDARY, BORDER_PRIMARY, BORDER_SECONDARY, FONT_MONO, FONT_SIZE_BASE, FONT_SIZE_LG, FONT_SIZE_MD,
    FONT_SIZE_XL, RADIUS_MD, SPACING_LG, SPACING_MD, SPACING_SM, TEXT_ACCENT, TEXT_PRIMARY, TEXT_SECONDARY,
};
use dioxus::prelude::*;

#[component]
pub fn PlayerResourcesView(state: crate::state::State) -> Element {
    if state.aggregated.is_empty() {
        return rsx! {
            div { style: "background: {BG_SECONDARY}; border: 1px solid {BORDER_PRIMARY}; border-radius: {RADIUS_MD}; padding: {SPACING_LG};",
                h2 { style: "color: {TEXT_PRIMARY}; font-size: {FONT_SIZE_XL}; font-weight: 600; margin: 0 0 {SPACING_MD} 0;",
                    "Player Resources"
                }
                div { style: "display: flex; align-items: center; justify-content: center; padding: 40px 0; color: {TEXT_SECONDARY}; font-size: {FONT_SIZE_LG};",
                    "No player data available"
                }
            }
        };
    }

    rsx! {
        div { style: "background: {BG_SECONDARY}; border: 1px solid {BORDER_PRIMARY}; border-radius: {RADIUS_MD}; padding: {SPACING_LG};",
            h2 { style: "color: {TEXT_PRIMARY}; font-size: {FONT_SIZE_XL}; font-weight: 600; margin: 0 0 {SPACING_MD} 0;",
                "Player Resources"
            }
            div { style: "display: flex; flex-direction: column; gap: {SPACING_MD}; max-height: 600px; overflow-y: auto;",
                for (steam_id , resources) in &state.aggregated {
                    div { style: "background: {BG_PRIMARY}; border: 1px solid {BORDER_SECONDARY}; border-radius: {RADIUS_MD}; padding: {SPACING_MD};",
                        div { style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: {SPACING_SM}; line-height: 1.8;",
                            h3 { style: "color: {TEXT_ACCENT}; font-size: {FONT_SIZE_LG}; font-weight: 600; margin: 0; font-family: {FONT_MONO};",
                                "{steam_id}"
                            }
                            div { style: "color: {TEXT_SECONDARY}; font-size: {FONT_SIZE_MD};",
                                {
                                    let count = resources.len();
                                    let suffix = if count != 1 { "s" } else { "" };
                                    format!("{count} resource{suffix}")
                                }
                            }
                        }
                        div { style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: {SPACING_SM};",
                            for (resource , amount) in resources {
                                div { style: "display: flex; justify-content: space-between; align-items: center; background: {BG_SECONDARY}; padding: 6px 10px; border-radius: 4px; border: 1px solid {BORDER_SECONDARY}; line-height: 1.8;",
                                    span { style: "color: {TEXT_PRIMARY}; font-size: {FONT_SIZE_BASE}; padding-top: 1px;",
                                        "{resource:?}"
                                    }
                                    span { style: "color: {TEXT_ACCENT}; font-size: {FONT_SIZE_BASE}; font-weight: 600; font-family: {FONT_MONO};",
                                        "{amount}"
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
