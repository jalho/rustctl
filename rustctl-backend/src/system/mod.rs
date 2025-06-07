use crate::core::CrossTasksSharedState;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub async fn monitor_usage(
    cancel: CancellationToken,
    interval: Duration,
    _shared: Arc<Mutex<CrossTasksSharedState>>,
) {
    let mut interval = tokio::time::interval(interval);
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                println!("Cancelled -- Cleaning up...");
                break;
            }
            _ = interval.tick() => {
            /*
             * TODO: Read CPU & memory usage on a regular interval and write to
             *       the cross-tasks shared state
             */
            }
        }
    }
    // TODO: Do cleanup
    println!("Cleanup done!");
}
