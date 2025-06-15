mod constants;
mod core;
mod game;
mod system;
mod web;

fn main() -> std::process::ExitCode {
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

    let _handle: log4rs::Handle = core::init_logging(log::LevelFilter::Debug);

    let cancel = tokio_util::sync::CancellationToken::new();

    let state = core::CrossTasksSharedState::init();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    let done: Result<(), NonRecoverableError> = runtime.block_on(async {
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
                cancel.child_token(),
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
                cancel.child_token(),
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
                cancel.child_token(),
                constants::INTERVAL_SYNC_CLIENT,
                listen_addr,
                tls_config,
                cors_allow_origin,
                state,
            ))
            .unwrap();
        log::info!("Web server started: {listen_addr}");

        /*
         * Activate cancellation sequences on SIGINT, SIGTERM etc.
         */
        let jh_signal = tokio::task::Builder::new()
            .name("wait_signal")
            .spawn(system::wait_signal(cancel))
            .unwrap();

        let done = tokio::select! {
            result = jh_monitor => result,
            result = jh_state => result,
            result = jh_web => result,
            result = jh_signal => result,
        };
        let done: Result<(), core::error::NonRecoverableError> = done.unwrap();
        return done;
    });

    match done {
        Err(err) => {
            /*
             * Logging of the error should be done near where it occurred.
             */
            let error: core::error::NonRecoverableError = err;
            let status: std::process::ExitCode = error.into();
            return status;
        }
        Ok(_) => {
            log::info!("Done");
            return std::process::ExitCode::SUCCESS;
        }
    }
}
