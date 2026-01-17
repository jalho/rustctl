type MakeService =
    axum::extract::connect_info::IntoMakeServiceWithConnectInfo<axum::Router, std::net::SocketAddr>;

pub async fn serve<A: tokio::net::ToSocketAddrs>(
    addr: A,
    tx: tokio::sync::mpsc::Sender<crate::ctl::Command>,
) {
    let tcp_listener: tokio::net::TcpListener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let router: axum::Router = axum::Router::new()
        .route("/", axum::routing::get(handler))
        .with_state(State::new(tx));

    let service: MakeService = router.into_make_service_with_connect_info::<std::net::SocketAddr>();

    axum::serve(tcp_listener, service).await.unwrap();
}

async fn handler(
    axum::extract::State(state): axum::extract::State<State>,
) -> axum::response::Response {
    state.tx.send(crate::ctl::Command::Reboot).await.unwrap();

    let payload: Vec<u8> = Vec::new();
    let body: axum::body::Body = payload.into();
    axum::response::Response::new(body)
}

#[derive(Clone)]
struct State {
    tx: tokio::sync::mpsc::Sender<crate::ctl::Command>,
}

impl State {
    fn new(tx: tokio::sync::mpsc::Sender<crate::ctl::Command>) -> Self {
        Self { tx }
    }
}
