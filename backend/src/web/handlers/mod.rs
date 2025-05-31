use super::WebServerState;
use crate::core::Client;
use axum::{
    extract::{ConnectInfo, State, WebSocketUpgrade},
    response::IntoResponse,
};
use std::{net::SocketAddr, sync::Arc};
use uuid::Uuid;

pub async fn ws_upgrade(
    ws: WebSocketUpgrade,
    connect_info: ConnectInfo<SocketAddr>,
    state: State<WebServerState>,
) -> impl IntoResponse {
    let shared_state = Arc::clone(&state.shared_state);
    ws.on_upgrade(async move |sock| {
        println!("Client accepted!");
        let client = Client::new(connect_info.0, sock, Arc::clone(&shared_state));
        let dummy_session = Uuid::new_v4();

        {
            let mut lock = shared_state.lock().await;
            lock.register(dummy_session.clone(), &client);
        }

        let _done: () = client.send_and_receive_messages().await;

        {
            let mut lock = shared_state.lock().await;
            lock.unregister(&dummy_session);
        }

        println!("Client dropped!");
    })
}
