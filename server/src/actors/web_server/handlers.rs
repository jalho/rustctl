pub async fn websocket_handler(
    ws: axum::extract::WebSocketUpgrade,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    axum::extract::State(state): axum::extract::State<crate::actors::web_server::State>,
) -> axum::response::Response {
    ws.on_upgrade(async move |socket| {
        let socket: axum::extract::ws::WebSocket = socket;
        let addr: std::net::SocketAddr = addr;
        let _state: crate::actors::web_server::State = state;

        log::debug!("Downstream client connected: {addr}");

        let (_tx, _rx) = futures_util::StreamExt::split(socket);

        log::debug!("Downstream client disconnected: {addr}");
    })
}
