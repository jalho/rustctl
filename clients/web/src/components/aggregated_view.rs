use dioxus::prelude::*;
use crate::State;

#[component]
pub fn AggregatedView(state: Option<State>) -> Element {
    let state = match state {
        Some(s) => s,
        None => return rsx!(
            p { "No aggregated data yet" }
        ),
    };

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
