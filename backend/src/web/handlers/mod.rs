use super::{ClientSession, WebServerState};
use crate::{constants::COOKIE_NAME_SESSION, core::Client};
use axum::{
    extract::{ConnectInfo, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::{
    SignedCookieJar,
    cookie::{self, Cookie},
};
use std::{net::SocketAddr, sync::Arc};
use uuid::Uuid;

/* TODO: Add CSRF protection? */
pub async fn login(jar: SignedCookieJar) -> impl IntoResponse {
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

    (StatusCode::OK, session).into_response()
}

pub async fn status(jar: SignedCookieJar) -> impl IntoResponse {
    match jar
        .get(COOKIE_NAME_SESSION)
        .and_then(|cookie| serde_json::from_str::<ClientSession>(cookie.value()).ok())
    {
        Some(_client_session) => (StatusCode::NO_CONTENT,).into_response(),
        None => StatusCode::UNAUTHORIZED.into_response(),
    }
}

pub async fn ws_upgrade(
    ws: WebSocketUpgrade,
    jar: SignedCookieJar,
    state: State<WebServerState>,
    connect_info: ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    match jar
        .get(COOKIE_NAME_SESSION)
        .and_then(|cookie| serde_json::from_str::<ClientSession>(cookie.value()).ok())
    {
        Some(client_session) => {
            let shared_state = Arc::clone(&state.shared_state);
            ws.on_upgrade(async move |sock| {
                let client = Client::new(connect_info.0, sock, Arc::clone(&shared_state));

                {
                    let mut lock = shared_state.lock().await;
                    lock.register(client_session.session_id, &client);
                }

                client.send_and_receive_messages().await;

                {
                    let mut lock = shared_state.lock().await;
                    lock.unregister(&client_session.session_id);
                }
            })
        }
        None => StatusCode::UNAUTHORIZED.into_response(),
    }
}
