use crate::core::CrossTasksSharedState;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

pub async fn monitor_usage(_interval: Duration, _shared: Arc<Mutex<CrossTasksSharedState>>) {
    /*
     * TODO: Read CPU & memory usage on a regular interval and write to the
     *       cross-tasks shared state
     */
}
