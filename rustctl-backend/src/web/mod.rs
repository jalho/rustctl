mod handlers;

use crate::{
    core::{CrossTasksSharedState, coroutines::Coroutine, error::NonRecoverableError},
    game::GameStateMachine,
};
use axum::{Router, routing};
use axum_server::tls_rustls::RustlsConfig;
use rustctl_common::{snapshot::StateTransitionInitiator, web_app::WEBSOCKET_CONNECT_URL_PATH};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;

pub async fn start(
    coroutine_identity: Coroutine,
    cancel: CancellationToken,
    shutdown_tx: tokio::sync::mpsc::Sender<Coroutine>,
    client_sync_interval: Duration,
    listen_addr: SocketAddr,
    tls_config: Option<RustlsConfig>,
    cors_allow_origin: String,
    shared: Arc<Mutex<CrossTasksSharedState>>,
) -> Result<(), NonRecoverableError> {
    let ip_hash_salt: String = generate_random_salt_not_secure();
    let app_state = WebServerState::init(client_sync_interval, shared, ip_hash_salt);

    let result: Result<(), NonRecoverableError>;
    {
        let mut lock = app_state.shared_state.lock().await;
        result = lock
            .game_state
            .update_and_launch(StateTransitionInitiator::AutomaticBySytem)
            .await;
    }
    if let Err(err) = result {
        log::info!("Requesting shutdown from coroutine {coroutine_identity}");
        shutdown_tx.send(coroutine_identity).await.unwrap();
        return Err(err);
    }

    let cors: CorsLayer = CorsLayer::new()
        .allow_origin(axum::http::HeaderValue::from_str(&cors_allow_origin).unwrap())
        .allow_credentials(true)
        .allow_methods([axum::http::Method::GET, axum::http::Method::OPTIONS])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    let router = Router::new()
        .route(
            WEBSOCKET_CONNECT_URL_PATH,
            routing::get(handlers::ws_upgrade),
        )
        .layer(cors)
        .with_state(app_state);

    let done: Option<Result<(), std::io::Error>> = match tls_config {
        Some(tls_config) => {
            let server = axum_server::bind_rustls(listen_addr, tls_config)
                .serve(router.into_make_service_with_connect_info::<SocketAddr>());
            cancel.run_until_cancelled(server).await
        }
        None => {
            let server = axum_server::bind(listen_addr)
                .serve(router.into_make_service_with_connect_info::<SocketAddr>());
            cancel.run_until_cancelled(server).await
        }
    };

    match done {
        Some(Err(err)) => todo!("{err}"),
        Some(Ok(_)) => todo!(),
        None => log::info!("Coroutine done: {coroutine_identity}"),
    }
    Ok(())
}

#[derive(Clone)]
pub struct WebServerState {
    client_sync_interval: Duration,
    ip_hash_salt: String,
    shared_state: Arc<Mutex<CrossTasksSharedState>>,
}

impl WebServerState {
    pub fn init(
        client_sync_interval: Duration,
        shared_state: Arc<Mutex<CrossTasksSharedState>>,
        ip_hash_salt: String,
    ) -> Self {
        Self {
            shared_state,
            client_sync_interval,
            ip_hash_salt,
        }
    }
}

fn generate_random_salt_not_secure() -> String {
    uuid::Uuid::new_v4().to_string()
}
