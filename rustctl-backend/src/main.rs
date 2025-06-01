mod constants;
mod core;
mod game;
mod system;
mod web;

fn main() {
    console_subscriber::init();

    let args = core::Cli::get_args();

    let (listen_addr, cors_allow_origin, tls_key_pem, tls_cert_pem) = match args.command {
        core::CliCommand::Start {
            listen_addr,
            cors_allow_origin,
            tls_key_pem,
            tls_cert_pem,
        } => (listen_addr, cors_allow_origin, tls_key_pem, tls_cert_pem),
    };

    let state = core::CrossTasksSharedState::init();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        let tls_config: Option<axum_server::tls_rustls::RustlsConfig> =
            match (tls_key_pem, tls_cert_pem) {
                (Some(key), Some(cert)) => {
                    let cert: std::vec::Vec<u8> = std::fs::read(cert).unwrap();
                    let key: std::vec::Vec<u8> = std::fs::read(key).unwrap();
                    let cfg = axum_server::tls_rustls::RustlsConfig::from_pem(cert, key)
                        .await
                        .unwrap();
                    Some(cfg)
                }
                _ => None,
            };

        /*
         * Monitor system resources's usage such as CPU and memory.
         */
        let jh_monitor = tokio::task::Builder::new()
            .name("monitor_usage")
            .spawn(system::monitor_usage(
                constants::INTERVAL_MONITOR_SYSTEM,
                state.clone(),
            ))
            .unwrap();

        /*
         * Read game state such as players's locations.
         */
        let jh_state = tokio::task::Builder::new()
            .name("read_state")
            .spawn(game::read_state(
                constants::INTERVAL_FETCH_GAME_STATE,
                state.clone(),
            ))
            .unwrap();

        /*
         * Serve a web app for observing and managing the system.
         */
        let jh_web = tokio::task::Builder::new()
            .name("web_server")
            .spawn(web::start(
                constants::INTERVAL_SYNC_CLIENT,
                listen_addr,
                tls_config,
                cors_allow_origin,
                state,
            ))
            .unwrap();

        jh_monitor.await.unwrap();
        jh_state.await.unwrap();
        jh_web.await.unwrap();
    });
}
