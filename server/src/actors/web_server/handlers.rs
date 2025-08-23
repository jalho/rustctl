pub async fn websocket_handler(
    ws: axum::extract::WebSocketUpgrade,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    axum::extract::State(state): axum::extract::State<crate::actors::web_server::State>,
) -> axum::response::Response {
    ws.on_upgrade(async move |socket| {
        let socket: axum::extract::ws::WebSocket = socket;
        let addr: std::net::SocketAddr = addr;
        let state: crate::actors::web_server::State = state;

        log::debug!("Downstream client connected: {addr}");

        let (_tx, _rx) = futures_util::StreamExt::split(socket);

        /*
         * TODO: Remove this POC send of static command on connect! Instead,
         *       receive commands from the downstream client's rx end and send
         *       them!
         */
        match state
            .tx_cmd_collect
            .send(rustctl_common::command::DownstreamClientMessage::ServerSaveAndClose)
            .await
        {
            Ok(_) => log::debug!("sent mock command"),
            Err(err) => log::error!("failed to send mock command: {err}"),
        }

        /*
         * TODO: Receive state updates from Aggregator's broadcast channel and
         *       send them to the downstream client's tx end.
         */

        log::debug!("Downstream client disconnected: {addr}");
    })
}
