use super::WebServerState;
use crate::core::Client;
use axum::{
    extract::{ConnectInfo, State, WebSocketUpgrade},
    response::IntoResponse,
};
use std::{net::SocketAddr, sync::Arc};

pub async fn ws_upgrade(
    ws: WebSocketUpgrade,
    connect_info: ConnectInfo<SocketAddr>,
    state: State<WebServerState>,
) -> impl IntoResponse {
    let shared_state = Arc::clone(&state.shared_state);
    ws.on_upgrade(async move |sock| {
        let connected_at = chrono::Utc::now();
        log::info!("Client accepted: {}", connect_info.0);
        let client = Client::new(
            connected_at,
            connect_info.0,
            sock,
            Arc::clone(&shared_state),
        );
        let _done: () = client
            .send_and_receive_messages(state.client_sync_interval)
            .await;
        log::info!("Client dropped");
    })
}
