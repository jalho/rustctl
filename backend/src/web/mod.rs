mod handlers;

pub async fn serve(
    tx_cmd_from_web_client: tokio::sync::mpsc::Sender<crate::ctl::CommandFromWebClient>,
    mut db_client: crate::database::Client,
    rx_cmd_from_controller: std::sync::Arc<
        tokio::sync::Mutex<tokio::sync::mpsc::Receiver<crate::ctl::CommandFromController>>,
    >,
) {
    loop {
        let mut router: axum::Router<State> = axum::Router::new();

        /*
         * Public static web content routes.
         */
        router = router.route("/", axum::routing::get(handlers::web));
        router = router.route("/favicon.ico", axum::routing::get(handlers::favicon));

        /*
         * Pre-auth routes.
         */
        router = router.route(
            "/poc/cookie/set",
            axum::routing::post(handlers::poc_set_cookie_signed),
        );
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
        /*
         * Post-auth routes.
         *
         * TODO: Add access control.
         */
        router = router.route(
            "/poc/cookie/require",
            axum::routing::post(handlers::poc_require_cookie_signed),
        );
        router = router.route(
            "/cmd/system/reboot",
            axum::routing::post(handlers::reboot_system),
        );
        router = router.route(
            "/cmd/web/restart",
            axum::routing::post(handlers::restart_web_server),
        );

        /*
         * TLS server config.
         */
        const DOMAIN_NAME: &str = "rustctl.internal";
        let stored: Option<crate::database::queries::TlsPemSelected> =
            db_client.select_tls_pem_latest().await.unwrap();

        let (private_key_pem, certificate_pem): (String, String) = match stored {
            Some(n) => {
                let private_key_pem: String = n.private_key_pem;
                let certificate_pem: String = n.certificate_pem;
                (private_key_pem, certificate_pem)
            }
            None => {
                let mut params: rcgen::CertificateParams = rcgen::CertificateParams::default();

                params.distinguished_name = rcgen::DistinguishedName::new();
                params
                    .distinguished_name
                    .push(rcgen::DnType::CommonName, DOMAIN_NAME);
                params.subject_alt_names = vec![rcgen::SanType::DnsName(
                    DOMAIN_NAME.to_string().try_into().unwrap(),
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

                (private_key_pem, certificate_pem)
            }
        };

        let mut crypto_provider: rustls::crypto::CryptoProvider =
            rustls::crypto::aws_lc_rs::default_provider();
        crypto_provider.kx_groups = vec![
            /*
             * Enforce Post Quantum Cryptography (PQC) compliant algorithm.
             *
             * TODO: Use PQC (ML-DSA) for cert authentication too.
             */
            rustls::crypto::aws_lc_rs::kx_group::X25519MLKEM768,
        ];

        let server_cfg_builder: rustls::ConfigBuilder<rustls::ServerConfig, rustls::WantsVersions> =
            rustls::ServerConfig::builder_with_provider(crypto_provider.into());
        let server_cfg_builder: rustls::ConfigBuilder<rustls::ServerConfig, rustls::WantsVerifier> =
            server_cfg_builder
                .with_protocol_versions(&[&rustls::version::TLS13])
                .unwrap();
        let client_cert_verifier: std::sync::Arc<
            dyn rustls::server::danger::ClientCertVerifier + 'static,
        > = rustls::server::WebPkiClientVerifier::no_client_auth();
        let server_cfg_builder: rustls::ConfigBuilder<
            rustls::ServerConfig,
            rustls::server::WantsServerCert,
        > = server_cfg_builder.with_client_cert_verifier(client_cert_verifier);

        let private_key: rustls::pki_types::PrivateKeyDer =
            <rustls::pki_types::PrivateKeyDer as rustls_pki_types::pem::PemObject>::from_pem_slice(
                private_key_pem.as_bytes(),
            )
            .unwrap();

        let certificate: rustls_pki_types::CertificateDer<'static> =
            <rustls::pki_types::CertificateDer as rustls_pki_types::pem::PemObject>::from_pem_slice(certificate_pem.as_bytes()).unwrap();
        let certificates: Vec<rustls::pki_types::CertificateDer> = vec![certificate];
        let server_cfg: rustls::ServerConfig = server_cfg_builder
            .with_single_cert(certificates, private_key)
            .unwrap();

        let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().unwrap();

        let router: axum::Router = router.with_state(State::new(
            tx_cmd_from_web_client.clone(),
            DOMAIN_NAME,
            addr.port(),
            db_client.clone(),
        ));

        let tls_acceptor: tokio_rustls::TlsAcceptor =
            tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(server_cfg));

        let tcp_listener: tokio::net::TcpListener =
            tokio::net::TcpListener::bind(addr).await.unwrap();
        log::info!("HTTPS server bound to {addr}");

        let router: std::sync::Arc<axum::Router> = std::sync::Arc::new(router);

        let task = tokio::spawn(handle_connections(
            tcp_listener,
            tls_acceptor,
            router.clone(),
            rx_cmd_from_controller.clone(),
        ));
        let _done = task.await;
    }
}

async fn handle_connections(
    tcp_listener: tokio::net::TcpListener,
    tls_acceptor: tokio_rustls::TlsAcceptor,
    router: std::sync::Arc<axum::Router>,
    rx_cmd_from_controller: std::sync::Arc<
        tokio::sync::Mutex<tokio::sync::mpsc::Receiver<crate::ctl::CommandFromController>>,
    >,
) {
    let mut rx_cmd_from_controller = rx_cmd_from_controller.lock().await;
    let http_server_builder: hyper::server::conn::http1::Builder =
        hyper::server::conn::http1::Builder::new();
    let graceful_shutdown: hyper_util::server::graceful::GracefulShutdown =
        hyper_util::server::graceful::GracefulShutdown::new();

    'accept_connections: loop {
        tokio::select! {
            conn = tcp_listener.accept() => {
                /*
                 * TCP connection.
                 */
                let (tcp_stream, _socket_addr): (tokio::net::TcpStream, std::net::SocketAddr) = conn.unwrap();
                log::debug!("TCP connection accepted: {tcp_stream:?}");

                /*
                 * TLS connection.
                 */
                let tls_stream: tokio_rustls::server::TlsStream<tokio::net::TcpStream> = match tls_acceptor.accept(tcp_stream).await {
                    Ok(n) => n,
                    Err(err) => {
                        log::error!("{err}", err = crate::get_full_error_message(&err));
                        continue 'accept_connections;
                    },
                };
                log::debug!("TLS connection accepted: {tls_stream:?}");

                /*
                 * Glue between a bunch of libraries.
                 */
                let io: hyper_util::rt::TokioIo<_> = hyper_util::rt::TokioIo::new(tls_stream);
                let service: hyper_util::service::TowerToHyperService<_>  = hyper_util::service::TowerToHyperService::new((*router).clone());
                let http_connection: hyper::server::conn::http1::Connection<_, _> = http_server_builder.serve_connection(io, service);

                /*
                 * Register connection for draining in graceful shutdown.
                 */
                let job = graceful_shutdown.watch(http_connection);

                let _task = tokio::spawn(job);
            }

            _ = rx_cmd_from_controller.recv() => {
                break 'accept_connections;
            }
        }
    }

    let connections: usize = graceful_shutdown.count();
    log::info!("HTTPS server shutting down ({connections} active connections)");
    graceful_shutdown.shutdown().await;
    if connections > 0 {
        log::info!("All connections drained");
    }
}

#[derive(Clone)]
struct State {
    tx: tokio::sync::mpsc::Sender<crate::ctl::CommandFromWebClient>,

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

    signing_keypair: std::sync::Arc<libcrux_ml_dsa::ml_dsa_87::MLDSA87KeyPair>,
}

impl State {
    fn new(
        tx: tokio::sync::mpsc::Sender<crate::ctl::CommandFromWebClient>,
        domain_name: &str,
        port: u16,
        db_client: crate::database::Client,
    ) -> Self {
        let (rp_id, rp_origin): (&str, url::Url) = {
            let rp_id: &str = domain_name;

            let rp_origin: url::Url = format!("https://{domain_name}:{port}").parse().unwrap();

            (rp_id, rp_origin)
        };

        let builder: webauthn_rs::WebauthnBuilder<'_> =
            webauthn_rs::WebauthnBuilder::new(rp_id, &rp_origin).expect("Invalid Webauthn Config");

        let webauthn: webauthn_rs::Webauthn = builder.build().expect("Failed to build Webauthn");

        /*
         * TODO: Store signing keypair in database?
         */
        let mut signing_keypair_seed: [u8; libcrux_ml_dsa::KEY_GENERATION_RANDOMNESS_SIZE] =
            [0u8; libcrux_ml_dsa::KEY_GENERATION_RANDOMNESS_SIZE];
        rand::TryRng::try_fill_bytes(&mut rand::rngs::SysRng, &mut signing_keypair_seed).unwrap();
        let signing_keypair: libcrux_ml_dsa::ml_dsa_87::MLDSA87KeyPair =
            libcrux_ml_dsa::ml_dsa_87::generate_key_pair(signing_keypair_seed);

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

            signing_keypair: std::sync::Arc::new(signing_keypair),
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

/*
 * Constants mandated by ML-DSA-87.
 */
const SIGNATURE_SIZE_BYTES: usize = 4627;
