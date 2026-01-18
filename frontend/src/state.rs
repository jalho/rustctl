#[derive(Clone)]
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

    pub async fn keep_connected() {
        let mut state: GlobalState = dioxus::hooks::use_context::<GlobalState>();

        loop {
            dioxus::signals::WritableExt::with_mut(&mut state.connection_attempts, |n| *n += 1);

            if let Ok(socket) = gloo_net::websocket::futures::WebSocket::open("/websocket") {
                Self::handle_socket(socket).await;
            } else {
                gloo_timers::future::sleep(std::time::Duration::from_secs(1)).await;
            };
        }
    }

    async fn handle_socket(socket: gloo_net::websocket::futures::WebSocket) {
        let (tx, mut rx): (WsTx, WsRx) = futures_util::StreamExt::split(socket);

        /*
         * Store a connected socket's writeable end's ref to the global state.
         */
        {
            let mut state = dioxus::hooks::use_context::<GlobalState>();
            dioxus::signals::WritableExt::set(&mut state.ws_tx, Some(tx));
        }

        while let Some(Ok(message)) = futures_util::StreamExt::next(&mut rx).await {
            Self::handle_message(message).await;
        }

        /*
         * Socket is now gone: Clear its ref from the global state.
         */
        {
            let mut state = dioxus::hooks::use_context::<GlobalState>();
            dioxus::signals::WritableExt::set(&mut state.ws_tx, None);
        }
    }

    async fn handle_message(_message: gloo_net::websocket::Message) {}
}

/// Writeable half of a WebSocket.
type WsTx = futures_util::stream::SplitSink<
    gloo_net::websocket::futures::WebSocket,
    gloo_net::websocket::Message,
>;

/// Readable half of a WebSocket.
type WsRx = futures_util::stream::SplitStream<gloo_net::websocket::futures::WebSocket>;
