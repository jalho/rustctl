use dioxus::prelude::*;

#[component]
pub fn PlayerResourcesView(state: crate::state::State) -> Element {
    if state.aggregated.is_empty() {
        return rsx! {
            div { style: "background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 16px;",
                h2 { style: "color: #e6edf3; font-size: 16px; font-weight: 600; margin: 0 0 12px 0;",
                    "Player Resources"
                }
                div { style: "display: flex; align-items: center; justify-content: center; padding: 40px 0; color: #7d8590; font-size: 14px;",
                    "No player data available"
                }
            }
        };
    }

    rsx! {
        div { style: "background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 16px; height: 100%;",
            h2 { style: "color: #e6edf3; font-size: 16px; font-weight: 600; margin: 0 0 12px 0;",
                "Player Resources"
            }
            div { style: "display: flex; flex-direction: column; gap: 12px; max-height: 600px; overflow-y: auto;",
                for (steam_id , resources) in &state.aggregated {
                    div { style: "background: #0d1117; border: 1px solid #21262d; border-radius: 6px; padding: 12px;",
                        div { style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px;",
                            h3 { style: "color: #58a6ff; font-size: 14px; font-weight: 600; margin: 0; font-family: monospace;",
                                "{steam_id}"
                            }
                            div { style: "color: #7d8590; font-size: 12px;",
                                {
                                    let count = resources.len();
                                    let suffix = if count != 1 { "s" } else { "" };
                                    format!("{count} resource{suffix}")
                                }
                            }
                        }
                        div { style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 8px;",
                            for (resource , amount) in resources {
                                div { style: "display: flex; justify-content: space-between; align-items: center; background: #161b22; padding: 6px 10px; border-radius: 4px; border: 1px solid #21262d;",
                                    span { style: "color: #e6edf3; font-size: 13px;",
                                        "{resource:?}"
                                    }
                                    span { style: "color: #58a6ff; font-size: 13px; font-weight: 600; font-family: monospace;",
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
