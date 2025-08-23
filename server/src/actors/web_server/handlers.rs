type DownstreamSink = futures_util::stream::SplitSink<axum::extract::ws::WebSocket, axum::extract::ws::Message>;
type DownstreamStream = futures_util::stream::SplitStream<axum::extract::ws::WebSocket>;

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

        let rx_broadcast: tokio::sync::broadcast::Receiver<rustctl_common::snapshot::Snapshot> =
            state.tx_broadcast.subscribe();
        let tx_cmd_collect: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage> =
            state.tx_cmd_collect.clone();
        let (sink, stream): (DownstreamSink, DownstreamStream) = futures_util::StreamExt::split(socket);

        let job_collect = collect_messages_from_downstream(stream, tx_cmd_collect);
        let job_send = send_updates_to_downstream(sink, rx_broadcast);

        let _done: () = tokio::select! {
            n = job_collect => n,
            n = job_send => n,
        };

        log::debug!("Downstream client disconnected: {addr}");
    })
}

async fn collect_messages_from_downstream(
    mut stream: DownstreamStream,
    tx_cmd_collect: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
) -> () {
    /*
     * TODO: In a loop, read from `stream` and write to `tx_cmd_collect`
     */
    loop {
        let msg_raw: axum::extract::ws::Message = match futures_util::StreamExt::next(&mut stream).await {
            Some(Ok(n)) => n,
            Some(Err(err)) => {
                log::debug!("Downstream client stream closed: {err}");
                return ();
            }
            None => {
                log::debug!("Downstream client stream closed");
                return ();
            }
        };
        dbg!(msg_raw);
    }
}

async fn send_updates_to_downstream(
    mut sink: DownstreamSink,
    mut rx_broadcast: tokio::sync::broadcast::Receiver<rustctl_common::snapshot::Snapshot>,
) {
    loop {
        let snapshot: rustctl_common::snapshot::Snapshot = match rx_broadcast.recv().await {
            Ok(n) => n,
            Err(_err) => todo!(),
        };

        let serialized: String = match serde_json::to_string(&snapshot) {
            Ok(n) => n,
            Err(_err) => todo!(),
        };
        let serialized: axum::extract::ws::Utf8Bytes = serialized.into();

        let msg: axum::extract::ws::Message = axum::extract::ws::Message::Text(serialized);
        if let Err(_err) = futures_util::SinkExt::send(&mut sink, msg).await {
            todo!();
        }
    }
}
