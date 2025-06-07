use crate::core::CrossTasksSharedState;
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
) {
    let mut interval = tokio::time::interval(interval);
    loop {
        let is_cancelled: bool = cancel.is_cancelled();
        if is_cancelled {
            println!("Cancelled -- Cleaning up...");
            break;
        } else {
            interval.tick().await;
            /*
             * TODO: Read CPU & memory usage on a regular interval and write to
             *       the cross-tasks shared state
             */
        }
    }
    // TODO: Do cleanup
    println!("Cleanup done!");
}

pub async fn wait_signal(cancel: CancellationToken) {
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigint.recv() => println!("Got signal: SIGINT"),
        _ = sigterm.recv() => println!("Got signal: SIGTERM"),
    }
    cancel.cancel();
}
