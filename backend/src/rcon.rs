pub async fn relay(game_server_params: &crate::game::GameServerParameters) {
    let rcon_url: String = format!(
        "ws://127.0.0.1:{}/{}",
        game_server_params.rcon_port, game_server_params.rcon_password
    );

    loop {
        match tokio_tungstenite::connect_async(&rcon_url).await {
            Ok((ws_stream, _)) => {
                log::info!("RCON connected to game server");
                handle_connection(ws_stream).await;
            }
            Err(err) => {
                log::warn!("Failed to connect to RCON (retrying in 5s): {err}");
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn handle_connection(
    mut ws_stream: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) {
    let mut interval: tokio::time::Interval =
        tokio::time::interval(tokio::time::Duration::from_secs(10));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Err(err) = send_version_query(&mut ws_stream).await {
                    log::error!("Failed to send RCON query: {err}");
                    break;
                }
            }
            msg = futures_util::StreamExt::next(&mut ws_stream) => {
                match msg {
                    Some(Ok(tokio_tungstenite::tungstenite::protocol::Message::Text(text))) => {
                        process_message(text.to_string());
                    }
                    Some(Err(e)) => {
                        log::error!("RCON stream error: {e}");
                        break;
                    }
                    None => {
                        log::warn!("RCON connection closed by server");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn send_version_query(
    ws_stream: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    let query = RconPayload {
        Identifier: 1,
        Message: "c.version".to_string(),
    };

    let json_query: String = serde_json::to_string(&query).unwrap();
    futures_util::SinkExt::send(
        ws_stream,
        tokio_tungstenite::tungstenite::protocol::Message::Text(json_query.into()),
    )
    .await
}

fn process_message(text: String) {
    if let Ok(payload) = serde_json::from_str::<RconPayload>(&text) {
        log::info!("[RCON Response] {}", payload.Message);
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[allow(non_snake_case)]
struct RconPayload {
    Identifier: i32,
    Message: String,
}
