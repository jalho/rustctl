mod handlers;

const DOMAIN_NAME: &str = "rust.turust.eu";

pub async fn serve(
    server_params: &WebServerParameters,
    tx_cmd_from_web_client: tokio::sync::mpsc::Sender<crate::ctl::CommandFromWebClient>,
    db_client: crate::database::Client,
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
         * TODO(FEAT-1): Add access control.
         */
        router = router.route(
            "/poc/cookie/require",
            axum::routing::post(handlers::poc_require_cookie_signed),
        );

        /*
         * TLS server config.
         */
        let instant: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        let stored: Option<crate::database::queries::TlsPemSelected> = db_client
            .select_tls_pem_latest_valid_for(&instant)
            .await
            .unwrap();

        let (private_key_pem, certificate_chain_pem): crate::crypto::KeyPairPEM = match stored {
            Some(n) => {
                log::info!(
                    "Using existing TLS server certificate issued by {issuer} for {subject}, valid till {end}",
                    issuer = n.issuer_display,
                    subject = n.subject_display,
                    end = n.not_after,
                );
                let private_key_pem: String = n.private_key_pem;
                let certificate_chain_pem: String = n.certificate_chain_pem;
                (private_key_pem, certificate_chain_pem)
            }

            None => match server_params {
                WebServerParameters::TLSCertificateSelfSigned => {
                    log::info!("Generating self-signed TLS server certificate");

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
                            certificate_chain_pem: certificate_pem.to_owned(),
                        })
                        .await
                        .unwrap();

                    let cert_decoded: openssl::x509::X509 =
                        openssl::x509::X509::from_pem(certificate_pem.as_bytes()).unwrap();
                    log::info!(
                        "Using new self-signed TLS server certificate: [{not_before}, {not_after}]",
                        not_before = asn1_to_chrono(cert_decoded.not_before()),
                        not_after = asn1_to_chrono(cert_decoded.not_after()),
                    );

                    (private_key_pem, certificate_pem)
                }

                WebServerParameters::TLSCertificateLetsEncrypt => {
                    let (private_key_pem, cert_chain_pem): crate::crypto::KeyPairPEM =
                        provision_tls_certificate_via_acme(db_client.clone()).await;
                    db_client
                        .insert_one_tls_pem(crate::database::queries::TlsPemInsertable {
                            private_key_pem: private_key_pem.clone(),
                            certificate_chain_pem: cert_chain_pem.clone(),
                        })
                        .await
                        .unwrap();
                    (private_key_pem, cert_chain_pem)
                }
            },
        };

        let mut crypto_provider: rustls::crypto::CryptoProvider =
            rustls::crypto::aws_lc_rs::default_provider();
        crypto_provider.kx_groups = vec![
            /*
             * Require Post Quantum Cryptography (PQC) compliant algorithm
             * (ML-KEM) for TLS key exchange.
             *
             * NOTE: Currently not using PQC (ML-DSA) algorithms for TLS
             *       certificates's authentication. Maybe later.
             *       Cheatsheet: https://github.com/jalho/post-quantum-cryptography
             *
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

        let certificates: Vec<rustls::pki_types::CertificateDer> =
            <rustls::pki_types::CertificateDer as rustls_pki_types::pem::PemObject>::pem_slice_iter(
                certificate_chain_pem.as_bytes(),
            )
            .map(|result| result.unwrap())
            .collect();
        let server_cfg: rustls::ServerConfig = server_cfg_builder
            .with_single_cert(certificates, private_key)
            .unwrap();

        let addr: std::net::SocketAddr = "0.0.0.0:8080".parse().unwrap();

        let router: axum::Router = router.with_state(
            State::new(
                tx_cmd_from_web_client.clone(),
                DOMAIN_NAME,
                addr.port(),
                db_client.clone(),
            )
            .await,
        );

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

pub enum WebServerParameters {
    TLSCertificateSelfSigned,
    TLSCertificateLetsEncrypt,
}

async fn handle_connections(
    tcp_listener: tokio::net::TcpListener,
    tls_acceptor: tokio_rustls::TlsAcceptor,
    router: std::sync::Arc<axum::Router>,
    rx_cmd_from_controller: std::sync::Arc<
        tokio::sync::Mutex<tokio::sync::mpsc::Receiver<crate::ctl::CommandFromController>>,
    >,
) {
    /*
     * Lifecycle stuff: Shutdown gracefully on signal received via
     * `tokio::sync::mpsc::Receiver`.
     */
    let mut rx_cmd_from_controller = rx_cmd_from_controller.lock().await;
    let graceful_shutdown: hyper_util::server::graceful::GracefulShutdown =
        hyper_util::server::graceful::GracefulShutdown::new();

    /*
     * HTTP server options.
     */
    let mut http_server_builder: hyper::server::conn::http1::Builder =
        hyper::server::conn::http1::Builder::new();
    http_server_builder.keep_alive(false);

    'accept_connections: loop {
        tokio::select! {
            conn = tcp_listener.accept() => {
                /*
                 * TCP connection.
                 */
                let tcp_connection_accepted_at: tokio::time::Instant = tokio::time::Instant::now();
                let (tcp_stream, client_addr): (tokio::net::TcpStream, std::net::SocketAddr) = match conn {
                    Ok(n) => n,
                    Err(err) => {
                        let connection_age_millis: f64 = tcp_connection_accepted_at.elapsed().as_micros() as f64 / 1000.0;
                        log::warn!(
                            "TCP connection failed (age: TCP {connection_age_millis:.2} ms): {err}",
                            err = crate::get_full_error_message(&err),
                        );
                        continue 'accept_connections;
                    },
                };

                /*
                 * TLS connection.
                 */
                let tls_stream: tokio_rustls::server::TlsStream<tokio::net::TcpStream> = match tls_acceptor.accept(tcp_stream).await {
                    Ok(n) => n,
                    Err(err) => {
                        let connection_age_millis: f64 = tcp_connection_accepted_at.elapsed().as_micros() as f64 / 1000.0;
                        log::warn!(
                            "[client {client_addr}] TLS connection failed (age: TCP {connection_age_millis:.2} ms): {err}",
                            err = crate::get_full_error_message(&err),
                        );
                        continue 'accept_connections;
                    },
                };
                let tls_connection_accepted_at: tokio::time::Instant = tokio::time::Instant::now();

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

                let _task = tokio::spawn(async move {
                    let connection_done = job.await;
                    let connection_done_at: tokio::time::Instant = tokio::time::Instant::now();

                    let connection_age: tokio::time::Duration = connection_done_at.duration_since(tcp_connection_accepted_at);
                    let tls_phase: tokio::time::Duration = tls_connection_accepted_at.duration_since(tcp_connection_accepted_at);

                    let tls_percentage: f64 = {
                        let connection: f64 = connection_age.as_nanos() as f64;
                        let tls: f64 = tls_phase.as_nanos() as f64;
                        let percentage: f64 = tls / connection * 100.0;
                        percentage
                    };
                    let connection_age_millis: f64 = connection_age.as_micros() as f64 / 1000.0;

                    match connection_done {
                        Ok(_) => log::debug!(
                            "[client {client_addr}] Connection handled (age: TCP {connection_age_millis:.2} ms, TLS {tls_percentage:.2} %)"
                        ),
                        Err(err) => log::warn!(
                            "[client {client_addr}] Connection closed (age: TCP {connection_age_millis:.2} ms, TLS {tls_percentage:.2} %): {err}",
                            err = crate::get_full_error_message(&err),
                        ),
                    };
                });
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
    #[allow(dead_code)]
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
}

impl State {
    async fn new(
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

        match db_client.get_web_server_token_signing_key().await {
            Some(existing_key_pair) => {
                let _existing_key_pair: crate::crypto::KeyPairPEM = existing_key_pair;
            }
            None => {
                let web_server_token_signing_keypair: crate::crypto::KeyPairPEM =
                    crate::crypto::generate_web_server_token_signing_key_pair();
                db_client
                    .set_web_server_token_signing_key(web_server_token_signing_keypair)
                    .await;
                log::info!("Using new web server token signing keypair");
            }
        }

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

/// Acquire TLS certificate signed by "Let's Encrypt", using an ACME client.
async fn provision_tls_certificate_via_acme(
    db_client: crate::database::Client,
) -> crate::crypto::KeyPairPEM {
    /*
     * TODO(FEAT-0): Add support for the non-Staging ("production") URL
     */
    let lets_encrypt_url: String = instant_acme::LetsEncrypt::Staging.url().to_owned();
    log::info!(
        r#"Acquring TLS server certificate from "Let's Encrypt": {url}"#,
        url = lets_encrypt_url,
    );

    let builder: instant_acme::AccountBuilder = instant_acme::Account::builder().unwrap();

    let existing_credentials: Option<instant_acme::AccountCredentials> =
        db_client.get_acme_account_credentials().await;

    let account: instant_acme::Account = match existing_credentials {
        Some(credentials) => {
            let account: instant_acme::Account =
                builder.from_credentials(credentials).await.unwrap();
            log::info!(r#"Using existing ACME account for "Let's Encrypt""#);

            account
        }

        None => {
            let (account, credentials): (instant_acme::Account, instant_acme::AccountCredentials) =
                builder
                    .create(
                        &instant_acme::NewAccount {
                            contact: &[],
                            terms_of_service_agreed: true,
                            only_return_existing: false,
                        },
                        lets_encrypt_url,
                        None,
                    )
                    .await
                    .unwrap();
            db_client.set_acme_account_credentials(credentials).await;
            log::info!(r#"Created new ACME account for "Let's Encrypt""#);

            account
        }
    };

    let identifiers: &[instant_acme::Identifier; 1] =
        &[instant_acme::Identifier::Dns(String::from(DOMAIN_NAME))];

    let mut order: instant_acme::Order = account
        .new_order(&instant_acme::NewOrder::new(identifiers))
        .await
        .unwrap();

    let mut authorizations: instant_acme::Authorizations = order.authorizations();
    while let Some(result) = authorizations.next().await {
        let mut authz: instant_acme::AuthorizationHandle = result.unwrap();
        match authz.status {
            instant_acme::AuthorizationStatus::Pending => {}
            instant_acme::AuthorizationStatus::Valid => continue,
            _ => todo!(),
        }

        let mut challenge: instant_acme::ChallengeHandle = authz
            .challenge(instant_acme::ChallengeType::Http01)
            .unwrap();

        let authz_id: &instant_acme::AuthorizedIdentifier = challenge.identifier();
        log::debug!("{authz_id}");

        let key_authz: instant_acme::KeyAuthorization = challenge.key_authorization();
        let key_authz_http_01: &str = key_authz.as_str();

        /*
         * Serve "http-01" challenge
         */
        {
            let shutdown_flag_0: std::sync::Arc<std::sync::atomic::AtomicBool> =
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let shutdown_flag_1: std::sync::Arc<std::sync::atomic::AtomicBool> =
                shutdown_flag_0.clone();

            let listener: tokio::net::TcpListener =
                match tokio::net::TcpListener::bind("0.0.0.0:80").await {
                    Ok(l) => l,
                    Err(_) => todo!("port 80 already in use"),
                };

            let key_authz_http_01: String = key_authz_http_01.to_string();

            let challenge_server_task: tokio::task::JoinHandle<()> = tokio::spawn(async move {
                'serve_http_challenge: loop {
                    if shutdown_flag_1.load(std::sync::atomic::Ordering::Relaxed) {
                        break 'serve_http_challenge;
                    }

                    let (mut socket, _): (tokio::net::TcpStream, std::net::SocketAddr) =
                        match listener.accept().await {
                            Ok(n) => n,
                            Err(_) => continue 'serve_http_challenge,
                        };

                    let mut buf_inbound: Vec<u8> = vec![0u8; 4096];
                    let bytes_received: usize =
                        tokio::io::AsyncReadExt::read(&mut socket, &mut buf_inbound)
                            .await
                            .unwrap_or(0);
                    let buf_inbound_utf8: String =
                        String::from_utf8_lossy(&buf_inbound[..bytes_received]).to_string();
                    log::debug!("{buf_inbound_utf8}");

                    let buf_outbound_utf8: String = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
                        key_authz_http_01.len(),
                        key_authz_http_01
                    );

                    tokio::io::AsyncWriteExt::write_all(&mut socket, buf_outbound_utf8.as_bytes())
                        .await
                        .unwrap();
                    tokio::io::AsyncWriteExt::shutdown(&mut socket)
                        .await
                        .unwrap();
                }
            });

            challenge.set_ready().await.unwrap();

            shutdown_flag_0.store(true, std::sync::atomic::Ordering::Relaxed);
            let _ = challenge_server_task.await;
        }
    }

    let status: instant_acme::OrderStatus = order
        .poll_ready(&instant_acme::RetryPolicy::default())
        .await
        .unwrap();

    if status != instant_acme::OrderStatus::Ready {
        todo!();
    }

    let private_key_pem: String = order.finalize().await.unwrap();

    let cert_chain_pem: String = order
        .poll_certificate(&instant_acme::RetryPolicy::default())
        .await
        .unwrap();

    log::debug!("{}", private_key_pem);
    log::debug!("{}", cert_chain_pem);

    (private_key_pem, cert_chain_pem)
}
