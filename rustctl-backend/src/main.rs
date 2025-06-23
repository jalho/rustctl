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
    log::info!(
        "{name} {version}",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    );

    /*
     * Graceful shutdown mechanism: CancellationToken driven by:
     * - standard signals SIGINT, SIGTERM etc.
     * - shutdown channel that peer coroutines can use
     */
    let cancel = tokio_util::sync::CancellationToken::new();
    let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel::<core::coroutines::Coroutine>(1);

    let state: std::sync::Arc<tokio::sync::Mutex<core::CrossTasksSharedState>> =
        match core::CrossTasksSharedState::init() {
            Ok(n) => n,
            Err(err) => {
                /*
                 * Logging of the error should be done near where it occurred.
                 */
                let err: core::error::NonRecoverableError = err;
                let code = std::process::ExitCode::from(err);
                return code;
            }
        };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .worker_threads(1) // aiming for small footprint to leave maximal resources for the game
        .enable_all()
        .build()
        .unwrap();

    let done: Result<(), core::error::NonRecoverableError> = runtime.block_on(async {
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
            .name(&core::coroutines::Coroutine::MonitorUsage.to_string())
            .spawn(system::monitor_usage(
                core::coroutines::Coroutine::MonitorUsage,
                cancel.child_token(),
                shutdown_tx.clone(),
                constants::INTERVAL_MONITOR_SYSTEM,
                state.clone(),
            ))
            .unwrap();

        /*
         * Read game state such as players's locations.
         */
        let jh_state = tokio::task::Builder::new()
            .name(&core::coroutines::Coroutine::ReadState.to_string())
            .spawn(game::read_state(
                core::coroutines::Coroutine::ReadState,
                cancel.child_token(),
                shutdown_tx.clone(),
                constants::INTERVAL_FETCH_GAME_STATE,
                state.clone(),
            ))
            .unwrap();

        /*
         * Serve a web app for observing and managing the system.
         */
        let jh_web = tokio::task::Builder::new()
            .name(&core::coroutines::Coroutine::WebServer.to_string())
            .spawn(web::start(
                core::coroutines::Coroutine::WebServer,
                cancel.child_token(),
                shutdown_tx.clone(),
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
            .name(&core::coroutines::Coroutine::WaitSignal.to_string())
            .spawn(system::wait_signal(
                core::coroutines::Coroutine::WaitSignal,
                cancel,
                shutdown_rx,
            ))
            .unwrap();

        // coroutine results in terms of the runtime
        let tt_result_monitor = jh_monitor.await;
        let tt_result_state = jh_state.await;
        let tt_result_web = jh_web.await;
        let tt_result_signal = jh_signal.await;

        // results in terms of the application
        let res_monitor: Result<(), core::error::NonRecoverableError> = tt_result_monitor.unwrap();
        let res_state: Result<(), core::error::NonRecoverableError> = tt_result_state.unwrap();
        let res_web: Result<(), core::error::NonRecoverableError> = tt_result_web.unwrap();
        let res_signal: Result<(), core::error::NonRecoverableError> = tt_result_signal.unwrap();

        // return the NonRecoverableError if any of the tasks yielded it...
        for result in [res_monitor, res_state, res_web, res_signal] {
            if let Err(e) = result {
                return Err(e);
            }
        }
        // ...or else OK
        Ok(())
    });

    match done {
        Err(err) => {
            /*
             * Logging of the error should be done near where it occurred.
             */
            let error: core::error::NonRecoverableError = err;
            let status: std::process::ExitCode = error.into();
            status
        }
        Ok(_) => {
            log::info!("Done");
            std::process::ExitCode::SUCCESS
        }
    }
}
