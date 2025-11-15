use dioxus::prelude::*;

#[component]
pub fn CodeView(state: crate::state::State) -> Element {
    if let Ok(pretty) = serde_json::to_string_pretty(&state) {
        rsx!(
            div { style: "background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 16px;",
                div { style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px;",
                    h2 { style: "color: #e6edf3; font-size: 16px; font-weight: 600; margin: 0;",
                        "State Snapshot"
                    }
                    div { style: "color: #7d8590; font-size: 12px; font-family: monospace;",
                        "application/json"
                    }
                }
                div { style: "background: #0d1117; border: 1px solid #21262d; border-radius: 6px; padding: 16px; overflow-x: auto; max-height: 600px; overflow-y: auto;",
                    pre { style: "margin: 0; font-family: 'SF Mono', Monaco, Inconsolata, 'Roboto Mono', Consolas, 'Courier New', monospace; font-size: 12px; line-height: 1.6; color: #e6edf3;",
                        "{pretty}"
                    }
                }
            }
        )
    } else {
        rsx!(
            div { style: "background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 16px;",
                h2 { style: "color: #e6edf3; font-size: 16px; font-weight: 600; margin: 0 0 12px 0;",
                    "State Snapshot"
                }
                div { style: "display: flex; align-items: center; justify-content: center; padding: 40px 0; color: #f85149; font-size: 14px;",
                    "Failed to render snapshot"
                }
            }
        )
    }
}
