mod handlers;

pub struct WebServer {
    ctoken: tokio_util::sync::CancellationToken,
    tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

    listen_addr: (std::net::IpAddr, u16),
    router: axum::Router,
}

impl WebServer {
    pub fn new(
        ctoken: tokio_util::sync::CancellationToken,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

        listen_addr: (std::net::IpAddr, u16),

        tx_cmd_collect: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
        tx_broadcast: tokio::sync::broadcast::Sender<rustctl_common::BroadcastMessage>,
    ) -> Self {
        let state: State = State::init(tx_cmd_collect, tx_broadcast);

        let router: axum::Router = axum::Router::new()
            .route(
                rustctl_common::web_app::WEBSOCKET_CONNECT_URL_PATH,
                axum::routing::get(handlers::websocket_handler),
            )
            .route(
                rustctl_common::web_app::MAP_URL_PATH,
                axum::routing::get(handlers::map_handler),
            )
            .with_state(state);

        Self {
            ctoken,
            tx_activate,

            listen_addr,
            router,
        }
    }

    pub async fn work(self) -> Summary {
        let tcp_listener: tokio::net::TcpListener = match tokio::net::TcpListener::bind(&self.listen_addr).await {
            Ok(n) => n,
            Err(err) => {
                log::error!(
                    r#"Failed to bind TCP listener at "{host}:{port}": {err}"#,
                    host = self.listen_addr.0,
                    port = self.listen_addr.1,
                );
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
        Summary {}
    }
}

pub struct Summary {}

#[derive(Clone)]
pub struct State {
    tx_cmd_collect: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
    tx_broadcast: tokio::sync::broadcast::Sender<rustctl_common::BroadcastMessage>,
}

impl State {
    pub fn init(
        tx_cmd_collect: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
        tx_broadcast: tokio::sync::broadcast::Sender<rustctl_common::BroadcastMessage>,
    ) -> Self {
        Self {
            tx_cmd_collect,
            tx_broadcast,
        }
    }
}
