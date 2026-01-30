#[derive(Clone, Copy)]
pub struct GlobalState {
    pub connection_attempts: dioxus::prelude::Signal<u64>,
    pub ws_tx: dioxus::signals::Signal<Option<WsTx>>,
}

impl GlobalState {
    pub fn init() -> Self {
        Self {
            connection_attempts: dioxus::prelude::Signal::new(0),
            ws_tx: dioxus::signals::Signal::new(None),
        }
    }

    pub async fn keep_connected(mut state: GlobalState) {
        loop {
            dioxus::signals::WritableExt::with_mut(&mut state.connection_attempts, |n| *n += 1);

            if let Ok(socket) = gloo_net::websocket::futures::WebSocket::open("/websocket") {
                Self::handle_socket(state, socket).await;
            }

            let delay = *dioxus::signals::ReadableExt::read(&state.connection_attempts);
            let sleep_secs: u64 = std::cmp::max(1, std::cmp::min(delay, 10));
            gloo_timers::future::sleep(std::time::Duration::from_secs(sleep_secs)).await;
        }
    }

    async fn handle_socket(
        mut state: GlobalState,
        socket: gloo_net::websocket::futures::WebSocket,
    ) {
        dioxus::signals::WritableExt::set(&mut state.connection_attempts, 0);

        let (tx, mut rx): (WsTx, WsRx) = futures_util::StreamExt::split(socket);

        /*
         * Store the writeable end in global state.
         */
        dioxus::signals::WritableExt::set(&mut state.ws_tx, Some(tx));

        while let Some(Ok(message)) = futures_util::StreamExt::next(&mut rx).await {
            Self::handle_message(message).await;
        }

        /*
         * Socket is now gone: Clear its ref.
         */
        dioxus::signals::WritableExt::set(&mut state.ws_tx, None);
    }

    async fn handle_message(_message: gloo_net::websocket::Message) {
        // TODO
    }
}

/// Writeable half of a WebSocket.
type WsTx = futures_util::stream::SplitSink<
    gloo_net::websocket::futures::WebSocket,
    gloo_net::websocket::Message,
>;

/// Readable half of a WebSocket.
type WsRx = futures_util::stream::SplitStream<gloo_net::websocket::futures::WebSocket>;
