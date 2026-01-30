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

mod passkey {
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct RegistrationOptions {
        pub challenge: String,
        pub rp: Rp,
        pub user: User,
        pub pub_key_cred_params: Vec<PubKeyCredParam>,
        pub timeout: u64,
    }

    #[derive(serde::Serialize)]
    pub struct Rp {
        pub name: String,
        pub id: String,
    }

    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct User {
        pub id: String,
        pub name: String,
        pub display_name: String,
    }

    #[derive(serde::Serialize)]
    pub struct PubKeyCredParam {
        pub alg: i32,
        #[serde(rename = "type")]
        pub kind: String,
    }
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

mod handlers {
    pub async fn favicon() -> axum::response::Response {
        axum::response::Response::builder()
            .status(axum::http::StatusCode::NO_CONTENT)
            .body(axum::body::Body::empty())
            .unwrap()
    }

    pub async fn web() -> impl axum::response::IntoResponse {
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

    pub async fn reboot(
        axum::extract::State(state): axum::extract::State<super::State>,
    ) -> axum::response::Response {
        state.tx.send(crate::ctl::Command::Reboot).await.unwrap();

        let payload: Vec<u8> = Vec::new();
        let body: axum::body::Body = payload.into();
        axum::response::Response::new(body)
    }

    pub async fn auth_init(
        axum::extract::State(state): axum::extract::State<super::State>,
    ) -> axum::response::Json<crate::web::passkey::RegistrationOptions> {
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

        let options: crate::web::passkey::RegistrationOptions =
            crate::web::passkey::RegistrationOptions {
                challenge: challenge_b64,
                rp: crate::web::passkey::Rp {
                    name: "PLACEHOLDER1".into(),
                    id: "rustctl.internal".into(), // TODO: Use a public domain name
                },
                user: crate::web::passkey::User {
                    id: "PLACEHOLDER2".into(),
                    name: "PLACEHOLDER3".into(),
                    display_name: "PLACEHOLDER4".into(),
                },
                pub_key_cred_params: vec![crate::web::passkey::PubKeyCredParam {
                    alg: -7, // "ECDSA using P-256 and SHA-256"
                    kind: String::from("public-key"),
                }],
                timeout: 60000, // milliseconds
            };

        axum::response::Json(options)
    }
}
