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

    async fn to_scheme(&self, db_client: &mut crate::database::Client) -> Scheme {
        match self {
            Expose::LocalLoopback => return Scheme::Http,
            Expose::Any => {}
        }

        let stored: Option<crate::database::queries::TlsPemSelected> =
            db_client.select_tls_pem_latest().await.unwrap();

        let tls_config: axum_server::tls_rustls::RustlsConfig = match stored {
            Some(existing) => {
                let cert_decoded: openssl::x509::X509 =
                    openssl::x509::X509::from_pem(existing.certificate_pem.as_bytes()).unwrap();
                log::info!(
                    "Using existing TLS server certificate: [{not_before}, {not_after}]",
                    not_before = asn1_to_chrono(cert_decoded.not_before()),
                    not_after = asn1_to_chrono(cert_decoded.not_after()),
                );

                axum_server::tls_rustls::RustlsConfig::from_pem(
                    existing.certificate_pem.as_bytes().to_vec(),
                    existing.private_key_pem.as_bytes().to_vec(),
                )
                .await
                .unwrap()
            }

            None => {
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

                let private_key_pem: String = key_pair.serialize_pem();
                let certificate_pem: String = cert.pem();

                db_client
                    .insert_one_tls_pem(crate::database::queries::TlsPemInsertable {
                        private_key_pem: private_key_pem.to_owned(),
                        certificate_pem: certificate_pem.to_owned(),
                    })
                    .await
                    .unwrap();

                let cert_decoded: openssl::x509::X509 =
                    openssl::x509::X509::from_pem(certificate_pem.as_bytes()).unwrap();
                log::info!(
                    "Using new TLS server certificate: [{not_before}, {not_after}]",
                    not_before = asn1_to_chrono(cert_decoded.not_before()),
                    not_after = asn1_to_chrono(cert_decoded.not_after()),
                );

                axum_server::tls_rustls::RustlsConfig::from_pem(
                    certificate_pem.as_bytes().to_vec(),
                    private_key_pem.as_bytes().to_vec(),
                )
                .await
                .unwrap()
            }
        };

        Scheme::Https { tls_config }
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

/*
 * TODO: Why does the web server's in-mem state (see `pending_signups` and
 *       `pending_signins`) not reset when the web server (and its state) are
 *       recreated (following `rx_cmd_rws.recv()`)?
 *
 *       Architecture overview:
 *
 *       - Web server has an in-mem state (in addition to a database state). We
 *         are now concerned with why the in-mem state is not reset, even though
 *         it should, when the web server restarts.
 *
 *       - Web server has handler `restart_web_server` (`POST
 *         /cmd/web/restart`). It sends a `ctl::Command` to Controller via
 *         `tx_a`.
 *
 *       - When the Controller receives the `ctl::Command`, it relays a
 *         `ctl::CommandRWS` back to the web server via the sending half
 *         corresponding to `rx_cmd_rws`.
 *
 *       - Web server restarts when it receives a `ctl::CommandRWS` via the
 *         `rx_cmd_rws`.
 *
 *       Until this point, everything works as intended. Question is, again, why
 *       does the in-mem `State` of the web server persist between restarts?
 *
 *       Consider the following log evidence as of commit
 *       `f01171f5c19c227a9dd6ba2e9755a4b85166ba90`:
 *
 *       ```log
 *       14:53:02 UTC [INFO] rustctl v0.0.1 [backend/src/main.rs:24]
 *       14:53:02 UTC [INFO] Starting RCON WebSocket reconnect & handle loop with 5 second retry delay [backend/src/rcon.rs:8]
 *       14:53:02 UTC [INFO] Web server starting [backend/src/web/mod.rs:150]
 *       14:53:02 UTC [INFO] Connected to existing database [backend/src/database.rs:174]
 *       14:53:02 UTC [INFO] Using existing TLS server certificate: [1975-01-01 00:00:00 UTC, 4096-01-01 00:00:00 UTC] [backend/src/web/mod.rs:29]
 *       14:53:06 UTC [INFO] [63638e69] New sign-up initiated: Pending sign-ups in total: 1 [backend/src/web/handlers.rs:149]
 *       14:53:08 UTC [INFO] [499a7fe8] New sign-up initiated: Pending sign-ups in total: 2 [backend/src/web/handlers.rs:149]
 *       14:53:08 UTC [INFO] [1ccffb34] New sign-up initiated: Pending sign-ups in total: 3 [backend/src/web/handlers.rs:149]
 *       14:53:11 UTC [INFO] [1d7158e3] New sign-in initiated: Pending sign-ins in total: 1 [backend/src/web/handlers.rs:240]
 *       14:53:11 UTC [INFO] [881c5c16] New sign-in initiated: Pending sign-ins in total: 2 [backend/src/web/handlers.rs:240]
 *       14:53:12 UTC [INFO] [ae5653ab] New sign-in initiated: Pending sign-ins in total: 3 [backend/src/web/handlers.rs:240]
 *       14:53:15 UTC [INFO] Web server stopped [backend/src/web/mod.rs:181]
 *       14:53:15 UTC [INFO] Web server starting [backend/src/web/mod.rs:150]
 *       14:53:15 UTC [INFO] Using existing TLS server certificate: [1975-01-01 00:00:00 UTC, 4096-01-01 00:00:00 UTC] [backend/src/web/mod.rs:29]
 *       14:53:19 UTC [INFO] [939890cb] New sign-up initiated: Pending sign-ups in total: 4 [backend/src/web/handlers.rs:149]
 *       14:53:20 UTC [INFO] [e19563a2] New sign-up initiated: Pending sign-ups in total: 5 [backend/src/web/handlers.rs:149]
 *       14:53:21 UTC [INFO] [bd880c43] New sign-up initiated: Pending sign-ups in total: 6 [backend/src/web/handlers.rs:149]
 *       14:53:22 UTC [INFO] [980ffa05] New sign-in initiated: Pending sign-ins in total: 4 [backend/src/web/handlers.rs:240]
 *       14:53:23 UTC [INFO] [0fe3baa4] New sign-in initiated: Pending sign-ins in total: 5 [backend/src/web/handlers.rs:240]
 *       14:53:23 UTC [INFO] [be96f7c3] New sign-in initiated: Pending sign-ins in total: 6 [backend/src/web/handlers.rs:240]
 *       ```
 */

pub async fn serve(
    expose: &Expose,
    tx: tokio::sync::mpsc::Sender<crate::ctl::Command>,
    db_client: crate::database::Client,
    rx_cmd_rws: &mut tokio::sync::mpsc::Receiver<crate::ctl::CommandRWS>,
) {
    loop {
        let mut db_client_a = db_client.clone();
        let db_client_b = db_client.clone();
        let tx_a = tx.clone();

        let job = async move || {
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
            router = router.route(
                "/cmd/system/reboot",
                axum::routing::post(handlers::reboot_system),
            );
            router = router.route(
                "/cmd/web/restart",
                axum::routing::post(handlers::restart_web_server),
            );

            log::info!("Web server starting");
            let scheme: Scheme = expose.to_scheme(&mut db_client_a).await;
            let addr: std::net::SocketAddr = expose.into();
            let router: axum::Router = router.with_state(State::new(
                tx_a,
                scheme.to_url_scheme(),
                expose.domain_name(),
                addr.port(),
                db_client_b,
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
        };

        tokio::select! {
            _ = job() => todo!(),
            n = rx_cmd_rws.recv() => {
                match n {
                    Some(_) => {},
                    None => todo!(),
                }
            }
        }
        log::info!("Web server stopped");
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

pub fn asn1_to_chrono(not_before: &openssl::asn1::Asn1TimeRef) -> chrono::DateTime<chrono::Utc> {
    let unix_time_asn1: openssl::asn1::TimeDiff = openssl::asn1::Asn1Time::from_unix(0)
        .unwrap()
        .diff(not_before)
        .unwrap();

    const SECONDS_IN_DAY: i64 = 24 * 60 * 60;

    let unix_time_secs: i64 =
        unix_time_asn1.days as i64 * SECONDS_IN_DAY + unix_time_asn1.secs as i64;

    let value: chrono::DateTime<chrono::Utc> =
        chrono::DateTime::from_timestamp_secs(unix_time_secs).unwrap();

    value
}
