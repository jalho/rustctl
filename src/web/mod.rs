use crate::{
    constants::{ADDR_WEB_SERVICE_LISTEN, URL_PATH_WEBSOCKET_CONNECT},
    core::{SharedState, handle_websocket_upgrade},
};
use axum::{Router, http::StatusCode, response::Html, routing};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};

pub async fn start(shared: Arc<Mutex<SharedState>>) {
    let web_service = Router::new()
        .route("/", routing::get(content_main))
        .route(
            URL_PATH_WEBSOCKET_CONNECT,
            routing::get(routing::get(handle_websocket_upgrade)),
        )
        .fallback(routing::get(no_content))
        .with_state(shared);

    let listener = TcpListener::bind(ADDR_WEB_SERVICE_LISTEN).await.unwrap();

    axum::serve(
        listener,
        web_service.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn no_content() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn content_main() -> Html<String> {
    let content: String = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <style>
        html {{
            background-color: #121212;
            color: #e0e0e0;
            font-family: sans-serif;
        }}
        body {{
            margin: 2em;
        }}
        button {{
            background-color: #1e1e1e;
            color: #ffffff;
            border: 1px solid #333;
            padding: 0.5em 1em;
            cursor: pointer;
        }}
        pre {{
            background-color: #1e1e1e;
            color: #c0c0c0;
            padding: 1em;
            border-radius: 0.5em;
            overflow-x: auto;
        }}
    </style>
</head>
<body>
    <button onclick="ws.send('some command')">Send some command</button>
    <pre><code id="output"></code></pre>
</body>
<script>
    const ws = new WebSocket("{path_sock}");
    const output = document.getElementById("output");
    ws.addEventListener("message", (message) => {{
        const text = JSON.stringify(JSON.parse(message.data), null, 2);
        output.textContent = text;
    }});
</script>
</html>"#,
        path_sock = URL_PATH_WEBSOCKET_CONNECT
    );

    Html(content)
}
