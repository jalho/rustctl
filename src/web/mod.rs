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

pub async fn content_main() -> Html<String> {
    let content = format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>rustctl</title>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <style>
    :root {{
      --bg: #0d1117;
      --panel: #161b22;
      --border: #30363d;
      --text: #c9d1d9;
      --muted: #8b949e;
      --link: #58a6ff;
      --code-bg: #161b22;
      --button-bg: #21262d;
      --button-hover: #30363d;
      --button-text: #c9d1d9;
      --header-height: 56px;
    }}

    body {{
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
      margin: 0;
      background-color: var(--bg);
      color: var(--text);
      line-height: 1.6;
    }}

    header {{
      background-color: var(--panel);
      color: var(--text);
      padding: 16px 24px;
      font-size: 18px;
      font-weight: 600;
      border-bottom: 1px solid var(--border);
    }}

    main {{
      max-width: 960px;
      margin: 32px auto;
      padding: 0 24px;
    }}

    .panel {{
      background-color: var(--panel);
      border: 1px solid var(--border);
      border-radius: 6px;
      padding: 24px;
    }}

    h2 {{
      font-size: 18px;
      font-weight: 600;
      border-bottom: 1px solid var(--border);
      padding-bottom: 6px;
      margin-top: 24px;
      margin-bottom: 16px;
    }}

    .button-grid {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
      gap: 16px;
      margin-bottom: 24px;
    }}

    .command-button {{
      background-color: var(--button-bg);
      color: var(--button-text);
      border: 1px solid var(--border);
      padding: 12px;
      font-size: 16px;
      font-weight: 500;
      border-radius: 6px;
      cursor: pointer;
      text-align: center;
    }}

    .command-button:hover {{
      background-color: var(--button-hover);
    }}

    table {{
      width: 100%;
      border-collapse: collapse;
      background-color: var(--code-bg);
      border: 1px solid var(--border);
      border-radius: 6px;
      overflow: hidden;
      margin-bottom: 16px;
    }}

    th, td {{
      padding: 12px 16px;
      text-align: left;
      border-bottom: 1px solid var(--border);
    }}

    th {{
      background-color: #1f242d;
      color: var(--text);
      font-weight: 600;
    }}

    td img {{
      width: 24px;
      height: 16px;
      vertical-align: middle;
      margin-right: 8px;
    }}

    .placeholder {{
      background-color: #0e141b;
      border: 2px dashed var(--border);
      border-radius: 6px;
      height: 400px;
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--muted);
      margin-bottom: 16px;
    }}

    .status-dot {{
      display: inline-block;
      width: 10px;
      height: 10px;
      border-radius: 50%;
      margin-right: 8px;
    }}

    .online {{
      background-color: #2ea043;
    }}

    .offline {{
      background-color: #6e7681;
    }}

    .steam-id {{
      color: var(--muted);
    }}
  </style>
</head>
<body>

<header>
  rustctl
</header>

<main>
  <div class="panel">

    <h2>Player Statistics</h2>
    <div class="placeholder">
      Player stats and recent event feed placeholder
    </div>

    <h2>Real-Time World Map</h2>
    <div class="placeholder">
      Real-time map rendering placeholder
    </div>

    <h2>Players Online</h2>
    <table>
      <thead>
        <tr>
          <th>Player Name</th>
          <th>Country</th>
          <th>Steam ID</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td><span class="status-dot online"></span>player123</td>
          <td><img src="https://flagcdn.com/w40/us.png" alt="US">United States</td>
          <td class="steam-id">76561198000000000</td>
        </tr>
        <tr>
          <td><span class="status-dot offline"></span>raiderX</td>
          <td><img src="https://flagcdn.com/w40/se.png" alt="SE">Sweden</td>
          <td class="steam-id">76561198000000001</td>
        </tr>
        <tr>
          <td><span class="status-dot online"></span>farmerJoe</td>
          <td><img src="https://flagcdn.com/w40/gb.png" alt="GB">UK</td>
          <td class="steam-id">76561198000000002</td>
        </tr>
      </tbody>
    </table>

    <h2>Dashboard</h2>
    <div class="button-grid">
      <button class="command-button" onclick="sendCommand('command_foo')">Command Foo</button>
      <button class="command-button" onclick="sendCommand('command_bar')">Command Bar</button>
    </div>

    <h2>System Resource Monitor</h2>
    <div class="placeholder">
      CPU / Memory / I/O usage graphs placeholder
    </div>

  </div>
</main>

<script>
  let ws = null;

  function connectWebSocket() {{
    ws = new WebSocket("{path_websocket_connect}");

    ws.onmessage = function (event) {{
      // Future message handling
    }};
  }}

  function sendCommand(cmd) {{
    if (ws && ws.readyState === WebSocket.OPEN) {{
      ws.send(JSON.stringify({{ type: "command", payload: cmd }}));
    }}
  }}

  window.addEventListener("load", connectWebSocket);
</script>

</body>
</html>
        "#,
        path_websocket_connect = URL_PATH_WEBSOCKET_CONNECT
    );

    Html(content)
}
