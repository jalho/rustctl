use crate::core::CrossTasksSharedState;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

pub async fn read_state(_interval: Duration, _shared: Arc<Mutex<CrossTasksSharedState>>) {
    /*
     * TODO: Read game state over RCON on a regular interval and write to the cross-tasks shared state
     */
}
