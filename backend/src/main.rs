mod constants;
mod core;
mod game;
mod system;
mod web;

fn main() {
    console_subscriber::init();

    let args = core::Cli::get_args();

    let (web_root, backend_host, frontend_host) = match args.command {
        core::CliCommand::Start {
            web_root,
            backend_host,
            frontend_host,
        } => (web_root, backend_host, frontend_host),
    };

    let state = core::SharedState::init();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    let sans: Vec<String> = vec![backend_host.to_cert_san(), frontend_host.to_cert_san()];
    let cert_and_key: rcgen::CertifiedKey = rcgen::generate_simple_self_signed(sans).unwrap();

    runtime.block_on(async {
        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem(
            cert_and_key.cert.pem().as_bytes().to_vec(),
            cert_and_key.key_pair.serialize_pem().as_bytes().to_vec(),
        )
        .await
        .unwrap();

        /*
         * Monitor system resources's usage such as CPU and memory.
         */
        let jh_monitor = tokio::task::Builder::new()
            .name("monitor_usage")
            .spawn(system::monitor_usage(state.clone()))
            .unwrap();

        /*
         * Read game state such as players's locations.
         */
        let jh_state = tokio::task::Builder::new()
            .name("read_state")
            .spawn(game::read_state(state.clone()))
            .unwrap();

        /*
         * Serve a web app for observing and managing the system.
         */
        let jh_web = tokio::task::Builder::new()
            .name("web_server")
            .spawn(web::start(tls_config, frontend_host, state, web_root))
            .unwrap();

        jh_monitor.await.unwrap();
        jh_state.await.unwrap();
        jh_web.await.unwrap();
    });
}
