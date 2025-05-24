mod handlers;

use crate::core::SharedState;
use axum::{Router, extract::FromRef, routing};
use axum_extra::extract::cookie::Key;
use axum_server::tls_rustls::RustlsConfig;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

pub async fn start(
    cookie_domain: Option<String>,
    listen_addr: SocketAddr,
    tls_config: Option<RustlsConfig>,
    cors_allow_origin: String,
    shared: Arc<Mutex<SharedState>>,
) {
    let cookie_sign_verif_key = Key::generate();
    let is_tls: bool = match tls_config {
        Some(_) => true,
        None => false,
    };
    let app_state = WebServerState::init(cookie_domain, is_tls, cookie_sign_verif_key, shared);

    let cors: CorsLayer = CorsLayer::new()
        .allow_origin(axum::http::HeaderValue::from_str(&cors_allow_origin).unwrap())
        .allow_credentials(true)
        .allow_methods([axum::http::Method::GET, axum::http::Method::OPTIONS])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    let router = Router::new()
        .route("/api/login", routing::get(handlers::login))
        .route("/api/websocket", routing::get(handlers::ws_upgrade))
        .route("/api/status", routing::get(handlers::status))
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

impl From<Url> for String {
    fn from(val: Url) -> Self {
        let scheme = match val.scheme {
            Scheme::Https => "https",
            Scheme::Http => "http",
        };
        format!(
            "{scheme}://{hostname}:{port}",
            hostname = val.authority.hostname,
            port = val.authority.port,
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

        if !hostname.is_empty() {
            Ok(Url {
                scheme,
                authority: Host {
                    hostname: hostname.to_owned(),
                    port,
                },
            })
        } else {
            Err(format!("invalid URL: \"{parseable}\""))
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientSession {
    pub session_id: Uuid,
}

#[derive(Clone)]
pub struct WebServerState {
    cookie_secure: bool,
    cookie_domain: Option<String>,

    session_sign_verif_key: Key,
    pub shared_state: Arc<Mutex<SharedState>>,
}

impl WebServerState {
    pub fn init(
        cookie_domain: Option<String>,
        cookie_secure: bool,
        session_sign_verif_key: Key,
        shared_state: Arc<Mutex<SharedState>>,
    ) -> Self {
        Self {
            session_sign_verif_key,
            shared_state,
            cookie_secure,
            cookie_domain,
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
