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
    router = router.route("/", axum::routing::get(handle_web));
    router = router.route("/favicon.ico", axum::routing::get(handle_favicon));
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
    router = router.route("/auth/init", axum::routing::post(handle_auth_init));
    router = router.route("/reboot", axum::routing::post(handle_reboot));

    let router: axum::Router = router.with_state(State::new(tx));

    let service: MakeService = router.into_make_service_with_connect_info::<std::net::SocketAddr>();

    axum::serve(tcp_listener, service).await.unwrap();
}

async fn handle_favicon() -> axum::response::Response {
    axum::response::Response::builder()
        .status(axum::http::StatusCode::NO_CONTENT)
        .body(axum::body::Body::empty())
        .unwrap()
}

async fn handle_web() -> impl axum::response::IntoResponse {
    let path = "/home/rustctl/rustctl/target/dx/frontend/release/web/public/index.html";
    match tokio::fs::read_to_string(path).await {
        Ok(html) => axum::response::Response::builder()
            .header("content-type", "text/html")
            .body(axum::body::Body::from(html))
            .unwrap(),
        Err(_) => axum::response::Response::builder()
            .status(axum::http::StatusCode::NOT_FOUND)
            .body(axum::body::Body::from("Frontend not found"))
            .unwrap(),
    }
}

async fn handle_reboot(
    axum::extract::State(state): axum::extract::State<State>,
) -> axum::response::Response {
    state.tx.send(crate::ctl::Command::Reboot).await.unwrap();

    let payload: Vec<u8> = Vec::new();
    let body: axum::body::Body = payload.into();
    axum::response::Response::new(body)
}

async fn handle_auth_init(
    axum::extract::State(state): axum::extract::State<State>,
) -> axum::response::Json<RegistrationOptions> {
    let mut challenge_bytes: [u8; 32] = [0u8; 32];
    {
        let mut generator: rand::prelude::ThreadRng = rand::rng();
        use rand::RngCore;
        generator.fill_bytes(&mut challenge_bytes);
    }

    let challenge_b64: String = <base64::engine::GeneralPurpose as base64::Engine>::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        challenge_bytes,
    );

    {
        let mut lock = state.pending_challenges.lock().await;
        lock.insert(challenge_b64.clone());
    }

    let options: RegistrationOptions = RegistrationOptions {
        challenge: challenge_b64,
        rp: Rp {
            name: "PLACEHOLDER1".into(),
            id: "rustctl.internal".into(), // TODO: Use a public domain name
        },
        user: User {
            id: "PLACEHOLDER2".into(),
            name: "PLACEHOLDER3".into(),
            display_name: "PLACEHOLDER4".into(),
        },
        pub_key_cred_params: vec![PubKeyCredParam {
            alg: -7, // "ECDSA using P-256 and SHA-256"
            kind: String::from("public-key"),
        }],
        timeout: 60000, // milliseconds
    };

    axum::response::Json(options)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RegistrationOptions {
    challenge: String,
    rp: Rp,
    user: User,
    pub_key_cred_params: Vec<PubKeyCredParam>,
    timeout: u64,
}

#[derive(serde::Serialize)]
struct Rp {
    name: String,
    id: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct User {
    id: String,
    name: String,
    display_name: String,
}

#[derive(serde::Serialize)]
struct PubKeyCredParam {
    alg: i32,
    #[serde(rename = "type")]
    kind: String,
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
