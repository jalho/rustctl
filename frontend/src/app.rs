use dioxus::dioxus_core;
use dioxus::document;
use dioxus::prelude::asset;
use dioxus::prelude::dioxus_elements;
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::manganis;

#[dioxus::prelude::component]
pub fn App() -> dioxus::core::Element {
    dioxus::prelude::use_future(connect_websocket);

    dioxus::prelude::rsx! {
        document::Link { rel: "stylesheet", href: asset!("/assets/main.css") }
        p { "Hello, world!" }
    }
}

async fn connect_websocket() {
    loop {
        if let Ok(mut socket) = gloo_net::websocket::futures::WebSocket::open("/websocket") {
            use futures_util::StreamExt;
            while (socket.next().await).is_some() {}
        }
        gloo_timers::future::sleep(std::time::Duration::from_secs(1)).await;
    }
}
