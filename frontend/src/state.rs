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

        'reconnect: loop {
            dioxus::signals::WritableExt::with_mut(&mut state.connection_attempts, |n| *n += 1);

            let socket: Socket = match gloo_net::websocket::futures::WebSocket::open("/websocket") {
                Ok(n) => n,
                Err(_) => continue 'reconnect,
            };

            Self::handle_socket_receive_messages(socket).await;

            gloo_timers::future::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    async fn handle_socket_receive_messages(mut socket: Socket) {
        use futures_util::StreamExt;

        match socket.next().await {
            Some(Ok(n)) => {
                let _n: gloo_net::websocket::Message = n;
            }
            Some(Err(err)) => {
                let _err: gloo_net::websocket::WebSocketError = err;
            }
            None => return,
        }
    }
}

type Socket = gloo_net::websocket::futures::WebSocket;
