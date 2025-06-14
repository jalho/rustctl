use crate::core::CrossTasksSharedState;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub async fn read_state(
    cancel: CancellationToken,
    interval: Duration,
    _shared: Arc<Mutex<CrossTasksSharedState>>,
) {
    let mut interval = tokio::time::interval(interval);
    loop {
        let is_cancelled: bool = cancel.is_cancelled();
        if is_cancelled {
            break;
        } else {
            interval.tick().await;
            /*
             * TODO: Read game state over RCON on a regular interval and write
             *       to the cross-tasks shared state
             */
        }
    }
    log::info!("Cancelled");
}

#[derive(Clone)]
pub struct Init;

#[derive(Clone)]
pub enum GameState {
    A(Init),
}

impl GameState {
    pub fn handle_client_message(self, msg: String) -> Self {
        /*
         * TODO:
         * 1. Check if commands are expected -- Ignore if not!
         * 2. Pick arguments from command if necessary
         * 3. Do transition: Return new state
         */
        return self;
    }
}
