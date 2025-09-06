use futures_util::StreamExt;
use gloo_net::websocket::{Message, futures::WebSocket};
use rustctl_common::BroadcastMessage;
use wasm_bindgen_futures::spawn_local;

use crate::config::WS_URL;
use crate::state::{clear_state, handle_incremental_event, handle_snapshot};

const RECONNECT_INTERVAL_SECS: u64 = 1;

pub fn start_websocket_connection() {
    spawn_local(async move {
        let interval = std::time::Duration::from_secs(RECONNECT_INTERVAL_SECS);

        loop {
            if let Err(_) = run_websocket_connection().await {
                // Connection failed or was lost, clear state and wait before retrying
                clear_state();
                gloo_timers::future::sleep(interval).await;
            }
        }
    });
}

async fn run_websocket_connection() -> Result<(), Box<dyn std::error::Error>> {
    let ws_url = format!("{}{}", WS_URL, rustctl_common::web_app::WEBSOCKET_CONNECT_URL_PATH);

    let ws = WebSocket::open(&ws_url)?;
    let (_tx, mut rx) = ws.split();

    while let Some(msg) = rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(msg) = serde_json::from_str::<BroadcastMessage>(&text) {
                    handle_broadcast_message(msg);
                }
            }
            Ok(_) => {
                // Handle other message types if needed
            }
            Err(e) => {
                return Err(Box::new(e));
            }
        }
    }

    Ok(())
}

fn handle_broadcast_message(msg: BroadcastMessage) {
    match msg {
        BroadcastMessage::Snapshot(snapshot) => {
            handle_snapshot(snapshot);
        }
        BroadcastMessage::EventIncremental(event) => {
            handle_incremental_event(event);
        }
    }
}
