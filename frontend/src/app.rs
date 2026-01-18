use dioxus::prelude::*;

#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/assets/main.css") }
        p { "Hello world!" }
    }
}
