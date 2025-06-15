use crate::core::{CrossTasksSharedState, error::NonRecoverableError};
use std::{sync::Arc, time::Duration};
use tokio::{
    signal::unix::{SignalKind, signal},
    sync::Mutex,
};
use tokio_util::sync::CancellationToken;

pub async fn monitor_usage(
    cancel: CancellationToken,
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
    log::info!("Cancelled");
    return Ok(());
}

pub async fn wait_signal(cancel: CancellationToken) -> Result<(), NonRecoverableError> {
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigint.recv() => log::info!("SIGINT"),
        _ = sigterm.recv() => log::info!("SIGTERM"),
    }
    cancel.cancel();
    return Ok(());
}
