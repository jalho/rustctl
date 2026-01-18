#[derive(Clone)]
pub struct GlobalState {
    pub connection_attempts: dioxus::prelude::Signal<u64>,
}

impl GlobalState {
    pub fn init() -> Self {
        Self {
            connection_attempts: dioxus::prelude::Signal::new(0),
        }
    }

    pub async fn connect_websocket() {
        let mut state = dioxus::hooks::use_context::<GlobalState>();
        loop {
            dioxus::signals::WritableExt::with_mut(&mut state.connection_attempts, |count| {
                *count += 1
            });
            if let Ok(mut socket) = gloo_net::websocket::futures::WebSocket::open("/websocket") {
                use futures_util::StreamExt;
                while (socket.next().await).is_some() {}
            }
            gloo_timers::future::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}
