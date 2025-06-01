mod handlers;

use crate::core::CrossTasksSharedState;
use axum::{Router, routing};
use axum_server::tls_rustls::RustlsConfig;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

pub async fn start(
    client_sync_interval: Duration,
    listen_addr: SocketAddr,
    tls_config: Option<RustlsConfig>,
    cors_allow_origin: String,
    shared: Arc<Mutex<CrossTasksSharedState>>,
) {
    let app_state = WebServerState::init(client_sync_interval, shared);

    let cors: CorsLayer = CorsLayer::new()
        .allow_origin(axum::http::HeaderValue::from_str(&cors_allow_origin).unwrap())
        .allow_credentials(true)
        .allow_methods([axum::http::Method::GET, axum::http::Method::OPTIONS])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    let router = Router::new()
        .route("/api/websocket", routing::get(handlers::ws_upgrade))
        .layer(cors)
        .with_state(app_state);

    match tls_config {
        Some(tls_config) => {
            let server = axum_server::bind_rustls(listen_addr, tls_config)
                .serve(router.into_make_service_with_connect_info::<SocketAddr>());
            let _done: () = server.await.unwrap();
        }
        None => {
            let server = axum_server::bind(listen_addr)
                .serve(router.into_make_service_with_connect_info::<SocketAddr>());
            let _done: () = server.await.unwrap();
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientSession {
    pub session_id: Uuid,
}

#[derive(Clone)]
pub struct WebServerState {
    client_sync_interval: Duration,
    shared_state: Arc<Mutex<CrossTasksSharedState>>,
}

impl WebServerState {
    pub fn init(
        client_sync_interval: Duration,
        shared_state: Arc<Mutex<CrossTasksSharedState>>,
    ) -> Self {
        Self {
            shared_state,
            client_sync_interval,
        }
    }
}
