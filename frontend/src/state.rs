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
        let mut state: GlobalState = dioxus::hooks::use_context::<GlobalState>();

        loop {
            dioxus::signals::WritableExt::with_mut(&mut state.connection_attempts, |n| *n += 1);

            match gloo_net::websocket::futures::WebSocket::open("/websocket") {
                Ok(socket) => Self::handle_socket_receive_messages(socket).await,
                Err(_err) => gloo_timers::future::sleep(std::time::Duration::from_secs(1)).await,
            };
        }
    }

    async fn handle_socket_receive_messages(mut socket: gloo_net::websocket::futures::WebSocket) {
        match futures_util::StreamExt::next(&mut socket).await {
            Some(Ok(n)) => {
                let _n: gloo_net::websocket::Message = n;
            }
            Some(Err(err)) => {
                let _err: gloo_net::websocket::WebSocketError = err;
            }
            None => (),
        }
    }
}
