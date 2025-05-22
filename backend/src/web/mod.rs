use crate::{
    constants::COOKIE_NAME_SESSION,
    core::{SharedState, handle_websocket_upgrade},
};
use axum::{
    Router,
    extract::FromRef,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing,
};
use axum_extra::extract::cookie::{self, Cookie, Key, SignedCookieJar};
use axum_server::tls_rustls::RustlsConfig;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientSession {
    pub session_id: Uuid,
}

#[derive(Clone)]
pub struct WebServerState {
    session_sign_verif_key: Key,
    pub shared_state: Arc<Mutex<SharedState>>,
}

impl WebServerState {
    pub fn init(session_sign_verif_key: Key, shared_state: Arc<Mutex<SharedState>>) -> Self {
        Self {
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

pub async fn start(
    tls_config_be: RustlsConfig,
    tls_config_fe: RustlsConfig,
    cors_allow_origin: Url,
    shared: Arc<Mutex<SharedState>>,
    web_root: PathBuf,
) {
    let cookie_sign_verif_key = Key::generate();
    let app_state = WebServerState::init(cookie_sign_verif_key, shared);

    let cors_layer: CorsLayer = CorsLayer::new()
        .allow_origin(
            axum::http::HeaderValue::from_str(&Into::<String>::into(cors_allow_origin)).unwrap(),
        )
        .allow_credentials(true)
        .allow_methods([axum::http::Method::GET, axum::http::Method::OPTIONS])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    let router_be = Router::new()
        .route("/api/login", routing::get(login))
        .route("/api/sock", routing::get(handle_websocket_upgrade))
        .route("/api/status", routing::get(status))
        .layer(cors_layer)
        .with_state(app_state);
    let addr_be = SocketAddr::from(([0, 0, 0, 0], 8081));
    let server_be = axum_server::bind_rustls(addr_be, tls_config_be)
        .serve(router_be.into_make_service_with_connect_info::<SocketAddr>());

    let router_fe = Router::new()
        .route("/favicon.ico", routing::get(no_content))
        .route_service("/", ServeFile::new(web_root.join("index.html")))
        .nest_service("/assets", ServeDir::new(web_root.join("assets")));
    let addr_fe = SocketAddr::from(([0, 0, 0, 0], 8080));
    let server_fe = axum_server::bind_rustls(addr_fe, tls_config_fe)
        .serve(router_fe.into_make_service_with_connect_info::<SocketAddr>());

    _ = tokio::join!(server_be, server_fe);
}

async fn no_content() -> StatusCode {
    StatusCode::NO_CONTENT
}

/* TODO: Add CSRF protection? */
async fn login(jar: SignedCookieJar) -> impl IntoResponse {
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

    let response: Response = (StatusCode::OK, session).into_response();

    response
}

async fn status(jar: SignedCookieJar) -> impl IntoResponse {
    match jar
        .get(COOKIE_NAME_SESSION)
        .and_then(|cookie| serde_json::from_str::<ClientSession>(cookie.value()).ok())
    {
        Some(_client_session) => {
            let response: Response = (StatusCode::NO_CONTENT,).into_response();

            response
        }
        None => StatusCode::UNAUTHORIZED.into_response(),
    }
}

#[derive(Clone)]
pub struct Host {
    hostname: String,
    port: u16,
}

#[derive(Clone)]
pub enum Scheme {
    Https,
    Http,
}

#[derive(Clone)]
pub struct Url {
    scheme: Scheme,
    authority: Host,
}

impl Url {
    /// SAN as in _Subject Alt Name_.
    pub fn to_cert_san(&self) -> String {
        self.authority.hostname.to_owned()
    }
}

impl Into<String> for Url {
    fn into(self) -> String {
        let scheme = match self.scheme {
            Scheme::Https => "https",
            Scheme::Http => "http",
        };
        format!(
            "{scheme}://{hostname}:{port}",
            hostname = self.authority.hostname,
            port = self.authority.port,
        )
    }
}

impl std::str::FromStr for Url {
    type Err = String;

    fn from_str(parseable: &str) -> Result<Url, Self::Err> {
        let mut remainder: &str = parseable;
        let mut scheme: Scheme = Scheme::Https;

        if remainder.starts_with("http://") {
            scheme = Scheme::Http;
            remainder = &remainder[7..];
        } else if remainder.starts_with("https://") {
            scheme = Scheme::Https;
            remainder = &remainder[8..];
        }

        while remainder.ends_with("/") {
            remainder = &remainder[..remainder.len() - 1];
        }

        let mut hostname: &str = remainder;
        let mut port: u16 = match scheme {
            Scheme::Http => 80,
            Scheme::Https => 443,
        };

        if let Some(delim_idx) = remainder.rfind(':') {
            let port_raw: &str = &remainder[delim_idx + 1..];
            if let Ok(parsed_port) = u16::from_str_radix(port_raw, 10) {
                port = parsed_port;
                hostname = &remainder[..delim_idx];
            }
        }

        if hostname.len() > 0 {
            return Ok(Url {
                scheme,
                authority: Host {
                    hostname: hostname.to_owned(),
                    port,
                },
            });
        } else {
            return Err(format!("invalid URL: \"{parseable}\""));
        }
    }
}
