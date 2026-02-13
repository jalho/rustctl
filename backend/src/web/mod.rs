mod handlers;

pub enum Expose {
    LocalLoopback,
    Any,
}

impl Expose {
    pub fn domain_name(&self) -> &str {
        match self {
            Expose::LocalLoopback => "localhost",
            Expose::Any => "rustctl.internal",
        }
    }

    async fn to_scheme(&self) -> Scheme {
        match self {
            Expose::LocalLoopback => Scheme::Http,

            Expose::Any => {
                let mut params: rcgen::CertificateParams = rcgen::CertificateParams::default();

                let domain_name: &str = self.domain_name();

                params.distinguished_name = rcgen::DistinguishedName::new();
                params
                    .distinguished_name
                    .push(rcgen::DnType::CommonName, domain_name);
                params.subject_alt_names = vec![rcgen::SanType::DnsName(
                    domain_name.to_string().try_into().unwrap(),
                )];

                let key_pair: rcgen::KeyPair = rcgen::KeyPair::generate().unwrap();
                let cert: rcgen::Certificate = params.self_signed(&key_pair).unwrap();

                let tls_config: axum_server::tls_rustls::RustlsConfig =
                    axum_server::tls_rustls::RustlsConfig::from_pem(
                        cert.pem().into_bytes(),
                        key_pair.serialize_pem().into_bytes(),
                    )
                    .await
                    .unwrap();

                Scheme::Https { tls_config }
            }
        }
    }
}

impl From<&Expose> for std::net::SocketAddr {
    fn from(value: &Expose) -> Self {
        match value {
            Expose::LocalLoopback => "127.0.0.1:8080".parse().unwrap(),
            Expose::Any => "0.0.0.0:8080".parse().unwrap(),
        }
    }
}

pub async fn serve(
    expose: &Expose,
    tx: tokio::sync::mpsc::Sender<crate::ctl::Command>,
    db_client: crate::database::Client,
) {
    let mut router: axum::Router<State> = axum::Router::new();

    /*
     * Public static web content routes.
     */
    router = router.route("/", axum::routing::get(handlers::web));
    router = router.route("/favicon.ico", axum::routing::get(handlers::favicon));

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

    let scheme: Scheme = expose.to_scheme().await;
    let addr: std::net::SocketAddr = expose.into();
    let router: axum::Router = router.with_state(State::new(
        tx,
        scheme.to_url_scheme(),
        expose.domain_name(),
        addr.port(),
        db_client,
    ));

    match scheme {
        Scheme::Https { tls_config } => axum_server::bind_rustls(addr, tls_config)
            .serve(router.into_make_service())
            .await
            .unwrap(),
        Scheme::Http => axum_server::bind(addr)
            .serve(router.into_make_service())
            .await
            .unwrap(),
    }
}

#[derive(Clone)]
struct State {
    tx: tokio::sync::mpsc::Sender<crate::ctl::Command>,

    db_client: crate::database::Client,

    webauthn: std::sync::Arc<webauthn_rs::Webauthn>,

    pending_signups: std::sync::Arc<
        tokio::sync::Mutex<
            std::collections::HashMap<uuid::Uuid, Timestamped<NamedPasskeyRegistration>>,
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
    fn new(
        tx: tokio::sync::mpsc::Sender<crate::ctl::Command>,
        scheme: &str,
        domain_name: &str,
        port: u16,
        db_client: crate::database::Client,
    ) -> Self {
        let (rp_id, rp_origin): (&str, url::Url) = {
            let rp_id: &str = domain_name;

            let rp_origin: url::Url = format!("{scheme}://{domain_name}:{port}").parse().unwrap();

            (rp_id, rp_origin)
        };

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

            db_client,
        }
    }
}

enum Scheme {
    Https {
        tls_config: axum_server::tls_rustls::RustlsConfig,
    },

    Http,
}

impl Scheme {
    pub fn to_url_scheme(&self) -> &str {
        match self {
            Scheme::Https { .. } => "https",
            Scheme::Http => "http",
        }
    }
}

#[derive(Clone, Debug)]
struct Timestamped<T: Clone> {
    timestamp: chrono::DateTime<chrono::Utc>,
    inner: T,
}

impl<T: Clone> Timestamped<T> {
    pub fn new(timestamp: &chrono::DateTime<chrono::Utc>, inner: T) -> Self {
        Self {
            timestamp: *timestamp,
            inner,
        }
    }
}

#[derive(Clone, Debug)]
struct NamedPasskeyRegistration {
    passkey_name: String,
    pkr: webauthn_rs::prelude::PasskeyRegistration,
}

impl NamedPasskeyRegistration {
    pub fn new(passkey_name: &str, pkr: webauthn_rs::prelude::PasskeyRegistration) -> Self {
        Self {
            passkey_name: passkey_name.to_owned(),
            pkr,
        }
    }
}
