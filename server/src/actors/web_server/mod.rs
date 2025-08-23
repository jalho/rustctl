mod handlers;

pub struct WebServer {
    ctoken: tokio_util::sync::CancellationToken,
    tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

    router: axum::Router,
    tx_cmd_collect: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
}

impl WebServer {
    pub fn new(
        ctoken: tokio_util::sync::CancellationToken,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
        tx_cmd_collect: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
    ) -> Self {
        let router: axum::Router = axum::Router::new()
            .route(
                rustctl_common::web_app::WEBSOCKET_CONNECT_URL_PATH,
                axum::routing::get(handlers::websocket_handler),
            )
            .with_state(State::init());

        Self {
            ctoken,
            tx_activate,

            router,

            tx_cmd_collect,
        }
    }

    pub async fn work(self) -> Summary {
        const LISTEN_ADDR: &str = "127.0.0.1:8080";
        let tcp_listener: tokio::net::TcpListener = match tokio::net::TcpListener::bind(LISTEN_ADDR).await {
            Ok(n) => n,
            Err(err) => {
                log::error!("Failed to bind TCP listener at {LISTEN_ADDR}: {err}");
                if let Err(err) = self
                    .tx_activate
                    .send(crate::actors::terminator::Activator::WebServer)
                    .await
                {
                    log::error!("Failed to request termination: {err}");
                }
                return Summary {};
            }
        };

        let service = self
            .router
            .into_make_service_with_connect_info::<std::net::SocketAddr>();

        let job = async { axum::serve(tcp_listener, service).await };

        let done = self.ctoken.run_until_cancelled(job).await;
        match done {
            Some(Ok(n)) => {
                let _n: () = n;
                log::error!("Web server job terminated unexpectedly");
                if let Err(err) = self
                    .tx_activate
                    .send(crate::actors::terminator::Activator::WebServer)
                    .await
                {
                    log::error!("Failed to request termination: {err}");
                }
            }
            Some(Err(err)) => {
                let err: std::io::Error = err;
                log::error!("Web server failed: {err}");
                if let Err(err) = self
                    .tx_activate
                    .send(crate::actors::terminator::Activator::WebServer)
                    .await
                {
                    log::error!("Failed to request termination: {err}");
                }
            }
            None => {
                log::debug!("Web server job cancelled");
            }
        }
        return Summary {};
    }
}

pub struct Summary {}

#[derive(Clone)]
pub struct State {}

impl State {
    pub fn init() -> Self {
        Self {}
    }
}
