use dioxus::prelude::*;
use futures_util::StreamExt;

#[derive(Clone, Copy, PartialEq)]
pub struct GlobalState {
    pub last_message: Signal<Option<gloo_net::websocket::Message>>,
    pub ws_tx: Signal<Option<WsTx>>,
}

impl GlobalState {
    pub fn init() -> Self {
        Self {
            last_message: Signal::new(None),
            ws_tx: Signal::new(None),
        }
    }

    pub async fn connect(mut state: GlobalState) {
        if let Ok(socket) = gloo_net::websocket::futures::WebSocket::open("/websocket") {
            let (tx, mut rx) = socket.split();

            state.ws_tx.set(Some(tx));

            while let Some(Ok(message)) = rx.next().await {
                state.handle_message(message);
            }

            state.ws_tx.set(None);
        }
    }

    fn handle_message(&mut self, message: gloo_net::websocket::Message) {
        self.last_message.set(Some(message));
    }
}

type WsTx = futures_util::stream::SplitSink<
    gloo_net::websocket::futures::WebSocket,
    gloo_net::websocket::Message,
>;
