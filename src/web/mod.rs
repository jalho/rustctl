use crate::{
    constants::ADDR_WEB_SERVICE_LISTEN,
    core::{SharedState, handle_websocket_upgrade},
};
use axum::{
    Router,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing,
};
use std::{net::SocketAddr, path::Path, sync::Arc};
use tokio::{fs, net::TcpListener, sync::Mutex};

pub async fn start(shared: Arc<Mutex<SharedState>>) {
    let web_service = Router::new()
        .route("/", routing::get(get_index_html))
        .route("/styles.css", routing::get(get_styles_css))
        .route("/script.js", routing::get(get_script_js))
        .route(
            "/sock",
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

async fn get_index_html() -> Response {
    let path: &Path = Path::new("./web/index.html");
    match fs::read_to_string(path).await {
        Ok(content) => {
            let mut response: Response = Html(content).into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("text/html; charset=utf-8"),
            );
            response
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn get_styles_css() -> Response {
    let path: &Path = Path::new("./web/styles.css");
    match fs::read_to_string(path).await {
        Ok(content) => {
            let mut response: Response = content.into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("text/css; charset=utf-8"),
            );
            response
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn get_script_js() -> Response {
    let path: &Path = Path::new("./web/script.js");
    match fs::read_to_string(path).await {
        Ok(content) => {
            let mut response: Response = content.into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/javascript; charset=utf-8"),
            );
            response
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
