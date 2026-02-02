mod handlers;

const DOMAIN_NAME: &str = "rustctl.internal";

pub async fn serve<A>(addr: A, tx: tokio::sync::mpsc::Sender<crate::ctl::Command>)
where
    A: axum_server::Address + Send + 'static,
    <A as axum_server::Address>::Stream:
        tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    let mut params: rcgen::CertificateParams = rcgen::CertificateParams::default();

    /*
     * TODO: Parameterize the server domain name.
     */
    params.distinguished_name = rcgen::DistinguishedName::new();
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, DOMAIN_NAME);
    params.subject_alt_names = vec![rcgen::SanType::DnsName(
        DOMAIN_NAME.to_string().try_into().unwrap(),
    )];

    let key_pair: rcgen::KeyPair = rcgen::KeyPair::generate().unwrap();
    let cert: rcgen::Certificate = params.self_signed(&key_pair).unwrap();

    let config: axum_server::tls_rustls::RustlsConfig =
        axum_server::tls_rustls::RustlsConfig::from_pem(
            cert.pem().into_bytes(),
            key_pair.serialize_pem().into_bytes(),
        )
        .await
        .unwrap();

    let mut router: axum::Router<State> = axum::Router::new();

    /*
     * Public static web content routes.
     */
    router = router.route("/", axum::routing::get(handlers::web));
    router = router.route("/favicon.ico", axum::routing::get(handlers::favicon));
    router = router.nest_service(
        "/assets",
        tower_http::services::ServeDir::new(
            "/home/rustctl/rustctl/target/dx/frontend/release/web/public/assets",
        ),
    );

    /*
     * Logic routes.
     *
     * TODO: Add access control to some of the logic routes (post-auth).
     */
    router = router.route(
        shared::SIGN_UP_CHALLENGE,
        axum::routing::post(handlers::auth_sign_up_challenge),
    );
    router = router.route(
        shared::SIGN_UP_SUBMIT,
        axum::routing::post(handlers::auth_sign_up_submit),
    );
    router = router.route(
        shared::SIGN_IN_CHALLENGE,
        axum::routing::post(handlers::auth_sign_in_challenge),
    );
    router = router.route(
        shared::SIGN_IN_SUBMIT,
        axum::routing::post(handlers::auth_sign_in_submit),
    );
    router = router.route("/reboot", axum::routing::post(handlers::reboot));

    let router: axum::Router = router.with_state(State::new(tx));

    axum_server::bind_rustls(addr, config)
        .serve(router.into_make_service())
        .await
        .unwrap();
}

#[derive(Clone)]
struct State {
    tx: tokio::sync::mpsc::Sender<crate::ctl::Command>,

    db: std::sync::Arc<tokio::sync::Mutex<crate::database::Client>>,

    webauthn: std::sync::Arc<webauthn_rs::Webauthn>,

    pending_signups: std::sync::Arc<
        tokio::sync::Mutex<
            std::collections::HashMap<
                uuid::Uuid,
                Timestamped<webauthn_rs::prelude::PasskeyRegistration>,
            >,
        >,
    >,

    pending_signins: std::sync::Arc<
        tokio::sync::Mutex<
            std::collections::HashMap<
                uuid::Uuid,
                Timestamped<webauthn_rs::prelude::DiscoverableAuthentication>,
            >,
        >,
    >,
}

impl State {
    fn new(tx: tokio::sync::mpsc::Sender<crate::ctl::Command>) -> Self {
        let rp_id: &str = DOMAIN_NAME;
        let rp_origin: url::Url = url::Url::parse("https://rustctl.internal:8080").unwrap();

        let builder: webauthn_rs::WebauthnBuilder<'_> =
            webauthn_rs::WebauthnBuilder::new(rp_id, &rp_origin).expect("Invalid Webauthn Config");

        let webauthn: webauthn_rs::Webauthn = builder.build().expect("Failed to build Webauthn");

        Self {
            tx,

            webauthn: std::sync::Arc::new(webauthn),
            pending_signups: std::sync::Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            pending_signins: std::sync::Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),

            db: std::sync::Arc::new(tokio::sync::Mutex::new(crate::database::Client::new())),
        }
    }
}

#[derive(Clone, Debug)]
struct Timestamped<T: Clone> {
    timestamp: u128,
    inner: T,
}

impl<T: Clone> Timestamped<T> {
    pub fn new(inner: T) -> Self {
        let timestamp: u128 = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        Self { timestamp, inner }
    }
}
