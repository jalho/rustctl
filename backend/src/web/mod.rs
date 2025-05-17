use crate::{
    constants::{ADDR_WEB_SERVICE_LISTEN, COOKIE_NAME_SESSION},
    core::{SharedState, handle_websocket_upgrade},
};
use axum::{Router, extract::FromRef, http::StatusCode, response::IntoResponse, routing};
use axum_extra::extract::cookie::{Cookie, Key, SignedCookieJar};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};
use tower_http::services::ServeDir;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientSession {
    pub session_id: Uuid,
}

#[derive(Clone)]
pub struct AppState {
    key: Key,
    pub shared: Arc<Mutex<SharedState>>,
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.key.clone()
    }
}

impl FromRef<AppState> for Arc<Mutex<SharedState>> {
    fn from_ref(state: &AppState) -> Self {
        state.shared.clone()
    }
}

pub async fn start(shared: Arc<Mutex<SharedState>>, web_root: PathBuf) {
    let key = Key::generate();
    let app_state = AppState { key, shared };

    let web_service = Router::new()
        .route(
            "/sock",
            routing::get(routing::get(handle_websocket_upgrade)),
        )
        .route("/favicon.ico", routing::get(routing::get(no_content)))
        .route("/login", routing::get(login))
        .fallback_service(ServeDir::new(web_root).append_index_html_on_directories(true))
        .with_state(app_state);

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

async fn login(jar: SignedCookieJar) -> impl IntoResponse {
    let session: ClientSession = ClientSession {
        session_id: Uuid::new_v4(),
    };

    let session: String = serde_json::to_string(&session).unwrap();

    let session: SignedCookieJar = jar.add(Cookie::new(COOKIE_NAME_SESSION, session));

    (StatusCode::OK, session)
}
