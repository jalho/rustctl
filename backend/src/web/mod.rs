use crate::{
    constants::ADDR_WEB_SERVICE_LISTEN,
    core::{SharedState, handle_websocket_upgrade},
};
use axum::{Router, http::StatusCode, routing};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};
use tower_http::services::ServeDir;

pub async fn start(shared: Arc<Mutex<SharedState>>, web_root: PathBuf) {
    let web_service = Router::new()
        .route(
            "/sock",
            routing::get(routing::get(handle_websocket_upgrade)),
        )
        .route("/favicon.ico", routing::get(routing::get(no_content)))
        .fallback_service(ServeDir::new(web_root).append_index_html_on_directories(true))
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
