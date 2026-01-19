pub async fn relay(game_server_params: &crate::game::GameServerParameters) {
    let rcon_url: String = format!(
        "ws://127.0.0.1:{}/{}",
        game_server_params.rcon_port, game_server_params.rcon_password
    );

    loop {
        match tokio_tungstenite::connect_async(&rcon_url).await {
            Ok((ws_stream, _)) => {
                handle_connection(ws_stream).await;
            }
            Err(err) => {
                log::warn!("Failed to connect to RCON (retrying in 5s): {err}");
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

type Ws =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
type WsTx = futures_util::stream::SplitSink<Ws, tokio_tungstenite::tungstenite::Message>;
type WsRx = futures_util::stream::SplitStream<Ws>;

async fn handle_connection(socket: Ws) {
    log::info!("RCON connected to game server");

    let (ws_write, ws_read): (WsTx, WsRx) = futures_util::StreamExt::split(socket);

    let read_handle = tokio::spawn(read_loop(ws_read));
    let write_handle = tokio::spawn(write_loop(ws_write));

    tokio::select! {
        _ = read_handle => log::warn!("RCON read task ended"),
        _ = write_handle => log::warn!("RCON write task ended"),
    };
}

async fn read_loop(mut receiver: WsRx) {
    while let Some(msg) = futures_util::StreamExt::next(&mut receiver).await {
        match msg {
            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                process_message(text.to_string());
            }
            Ok(_) => {}
            Err(err) => {
                log::error!("RCON stream error: {err}");
                break;
            }
        }
    }
    log::warn!("RCON connection closed by server");
}

async fn write_loop(mut sender: WsTx) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60 * 60));

    loop {
        interval.tick().await;

        let query = RconPayload {
            Identifier: 1,
            Message: "c.version".to_string(),
        };

        if let Ok(json_query) = serde_json::to_string(&query) {
            log::debug!("[RCON Outbound] {}", json_query);
            if let Err(err) = futures_util::SinkExt::send(
                &mut sender,
                tokio_tungstenite::tungstenite::Message::Text(json_query.into()),
            )
            .await
            {
                log::error!("Failed to send RCON query: {err}");
                break;
            }
        }
    }
}

fn process_message(text: String) {
    if let Ok(payload) = serde_json::from_str::<RconPayload>(&text) {
        log::debug!("[RCON Inbound] {}", payload.Message);
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[allow(non_snake_case)]
struct RconPayload {
    Identifier: i32,
    Message: String,
}
