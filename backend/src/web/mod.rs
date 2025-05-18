use crate::{
    constants::{ADDR_WEB_SERVICE_LISTEN, COOKIE_NAME_SESSION},
    core::{SharedState, handle_websocket_upgrade},
};
use axum::{
    Router,
    extract::{FromRef, State},
    http::{
        HeaderValue, StatusCode,
        header::{ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_ORIGIN},
    },
    response::{IntoResponse, Response},
    routing,
};
use axum_extra::extract::cookie::{self, Cookie, Key, SignedCookieJar};
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
pub struct WebServerState {
    pub cors_allow_origin: Arc<String>,
    session_sign_verif_key: Key,
    pub shared_state: Arc<Mutex<SharedState>>,
}

impl WebServerState {
    pub fn init(
        cors_allow_origin: String,
        session_sign_verif_key: Key,
        shared_state: Arc<Mutex<SharedState>>,
    ) -> Self {
        Self {
            cors_allow_origin: Arc::new(cors_allow_origin),
            session_sign_verif_key,
            shared_state,
        }
    }
}

impl FromRef<WebServerState> for Key {
    fn from_ref(state: &WebServerState) -> Self {
        state.session_sign_verif_key.clone()
    }
}

impl FromRef<WebServerState> for Arc<Mutex<SharedState>> {
    fn from_ref(state: &WebServerState) -> Self {
        state.shared_state.clone()
    }
}

pub async fn start(cors_allow_origin: String, shared: Arc<Mutex<SharedState>>, web_root: PathBuf) {
    let key = Key::generate();
    let app_state = WebServerState::init(cors_allow_origin, key, shared);

    let web_service = Router::new()
        .route(
            "/sock",
            routing::get(routing::get(handle_websocket_upgrade)),
        )
        .route("/favicon.ico", routing::get(routing::get(no_content)))
        .route("/login", routing::get(login))
        .route("/status", routing::get(status))
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

/* TODO: Add CSRF protection? */
async fn login(jar: SignedCookieJar, state: State<WebServerState>) -> impl IntoResponse {
    let session: ClientSession = ClientSession {
        session_id: Uuid::new_v4(),
    };

    let session: String = serde_json::to_string(&session).unwrap();

    let mut cookie: Cookie<'static> = Cookie::new(COOKIE_NAME_SESSION, session.clone());
    cookie.set_path("/");
    cookie.set_http_only(false);
    cookie.set_secure(true);
    cookie.set_same_site(cookie::SameSite::None);

    let session: SignedCookieJar = jar.add(cookie);

    let cors_allow_origin: String = state.cors_allow_origin.clone().to_string();

    let response: Response = (
        StatusCode::OK,
        [
            (
                ACCESS_CONTROL_ALLOW_ORIGIN,
                HeaderValue::from_str(&cors_allow_origin).unwrap(),
            ),
            (
                ACCESS_CONTROL_ALLOW_CREDENTIALS,
                HeaderValue::from_static("true"),
            ),
        ],
        session,
    )
        .into_response();

    response
}

async fn status(jar: SignedCookieJar, state: State<WebServerState>) -> impl IntoResponse {
    match jar
        .get(COOKIE_NAME_SESSION)
        .and_then(|cookie| serde_json::from_str::<ClientSession>(cookie.value()).ok())
    {
        Some(_client_session) => {
            let cors_allow_origin: String = state.cors_allow_origin.clone().to_string();
            let response: Response = (
                StatusCode::NO_CONTENT,
                [
                    (
                        ACCESS_CONTROL_ALLOW_ORIGIN,
                        HeaderValue::from_str(&cors_allow_origin).unwrap(),
                    ),
                    (
                        ACCESS_CONTROL_ALLOW_CREDENTIALS,
                        HeaderValue::from_static("true"),
                    ),
                ],
            )
                .into_response();

            response
        }
        None => StatusCode::UNAUTHORIZED.into_response(),
    }
}
