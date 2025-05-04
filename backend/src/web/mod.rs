use crate::{
    constants::ADDR_WEB_SERVICE_LISTEN,
    core::{SharedState, handle_websocket_upgrade},
};
use axum::{
    Router,
    extract::State,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing,
};
use std::{net::SocketAddr, sync::Arc};
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

async fn get_index_html(shared: State<Arc<Mutex<SharedState>>>) -> Response {
    let path: String;
    {
        let shared = shared.lock().await;
        path = shared.web_assets.abs_path_index_html.clone();
    }

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

async fn get_styles_css(shared: State<Arc<Mutex<SharedState>>>) -> Response {
    let path: String;
    {
        let shared = shared.lock().await;
        path = shared.web_assets.abs_path_styles_css.clone();
    }

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

async fn get_script_js(shared: State<Arc<Mutex<SharedState>>>) -> Response {
    let path: String;
    {
        let shared = shared.lock().await;
        path = shared.web_assets.abs_path_script_js.clone();
    }

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
