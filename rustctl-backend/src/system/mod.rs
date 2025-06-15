use crate::core::{CrossTasksSharedState, coroutines::Coroutine, error::NonRecoverableError};
use std::{sync::Arc, time::Duration};
use tokio::{
    signal::unix::{SignalKind, signal},
    sync::Mutex,
};
use tokio_util::sync::CancellationToken;

pub async fn monitor_usage(
    coroutine_identity: Coroutine,
    cancel: CancellationToken,
    _shutdown_tx: tokio::sync::mpsc::Sender<Coroutine>,
    interval: Duration,
    _shared: Arc<Mutex<CrossTasksSharedState>>,
) -> Result<(), NonRecoverableError> {
    let mut interval = tokio::time::interval(interval);
    loop {
        let is_cancelled: bool = cancel.is_cancelled();
        if is_cancelled {
            break;
        } else {
            interval.tick().await;
            /*
             * TODO: Read CPU & memory usage on a regular interval and write to
             *       the cross-tasks shared state
             */
        }
    }
    log::info!("Cancelled coroutine {coroutine_identity}");
    Ok(())
}

pub async fn wait_signal(
    _coroutine_identity: Coroutine,
    cancel: CancellationToken,
    mut shutdown_rx: tokio::sync::mpsc::Receiver<Coroutine>,
) -> Result<(), NonRecoverableError> {
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigint.recv() => log::info!("SIGINT"),
        _ = sigterm.recv() => log::info!("SIGTERM"),
        Some(coroutine) = shutdown_rx.recv() => log::info!("Shutdown requested by coroutine {coroutine}"),
    }
    cancel.cancel();
    Ok(())
}
