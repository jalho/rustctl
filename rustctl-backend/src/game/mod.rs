use crate::core::CrossTasksSharedState;
use rustctl_common::snapshot::Game;
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

pub trait GameStateMachine {
    fn handle_client_message(self, client_msg: String) -> Self;
}

impl GameStateMachine for Game {
    fn handle_client_message(self, client_msg: String) -> Self {
        // TODO: Check if a command is even expected at this time
        // TODO: Get args from command if necessary
        // TODO: Make a state transition: return new state
        match self {
            Game::A(state) => todo!(),
            Game::B(state) => todo!(),
            Game::C(state) => todo!(),
            Game::D(state) => todo!(),
            Game::E(state) => todo!(),
        }
    }
}
