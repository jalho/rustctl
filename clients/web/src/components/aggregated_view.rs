use dioxus::prelude::*;

#[component]
pub fn AggregatedView(state: crate::state::State) -> Element {
    if state.aggregated.is_empty() {
        rsx! {
            p { "Nothing." }
        }
    } else {
        rsx! {
            div {
                for (steam_id , resources) in &state.aggregated {
                    div { style: "margin-bottom: 10px;",
                        h3 { "Player {steam_id}" }
                        ul {
                            for (resource , amount) in resources {
                                li { "{resource:?}: {amount}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
