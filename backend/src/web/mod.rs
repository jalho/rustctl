mod handlers;
mod passkey;

type MakeService =
    axum::extract::connect_info::IntoMakeServiceWithConnectInfo<axum::Router, std::net::SocketAddr>;

pub async fn serve<A: tokio::net::ToSocketAddrs>(
    addr: A,
    tx: tokio::sync::mpsc::Sender<crate::ctl::Command>,
) {
    let tcp_listener: tokio::net::TcpListener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let mut router: axum::Router<State> = axum::Router::new();

    /*
     * Public static web content routes.
     */
    router = router.route("/", axum::routing::get(handlers::web));
    router = router.route("/favicon.ico", axum::routing::get(handlers::favicon));
    router = router.nest_service(
        "/assets",
        tower_http::services::ServeDir::new(format!(
            "{}/assets",
            "/home/rustctl/rustctl/target/dx/frontend/release/web/public"
        )),
    );

    /*
     * Logic routes.
     *
     * TODO: Add access control to some of the logic routes (post-auth).
     */
    router = router.route("/auth/init", axum::routing::post(handlers::auth_init));
    router = router.route("/reboot", axum::routing::post(handlers::reboot));

    let router: axum::Router = router.with_state(State::new(tx));

    let service: MakeService = router.into_make_service_with_connect_info::<std::net::SocketAddr>();

    axum::serve(tcp_listener, service).await.unwrap();
}

#[derive(Clone)]
struct State {
    tx: tokio::sync::mpsc::Sender<crate::ctl::Command>,
    pending_challenges: std::sync::Arc<tokio::sync::Mutex<std::collections::HashSet<String>>>,
}

impl State {
    fn new(tx: tokio::sync::mpsc::Sender<crate::ctl::Command>) -> Self {
        Self {
            tx,
            pending_challenges: std::sync::Arc::new(tokio::sync::Mutex::new(
                std::collections::HashSet::new(),
            )),
        }
    }
}
