use crate::core::CrossTasksSharedState;
use rustctl_common::{snapshot::Game, state_machine::NotRunning};
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

pub trait GameStateMachineCommand {
    async fn determine_initial_state(&mut self);
    async fn handle_client_message(&mut self, client_msg: String);
}

impl GameStateMachineCommand for Game {
    async fn determine_initial_state(&mut self) {
        // TODO: Check if SteamCMD or RustDedicated is already running
        *self = Game::B(NotRunning);
    }

    async fn handle_client_message(&mut self, client_msg: String) {
        // TODO: Check if a command is even expected at this time
        // TODO: Check if the received command matches the current state
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

mod proc {
    /// Absolute path of the `steamcmd` executable, i.e. the game server installer.
    const EXEC_ABS_STEAMCMD: &'static str = "/home/jka/probe/mock-steamcmd";
    /// Absolute path of the `RustDedicated` executable, i.e. the game server.
    const EXEC_ABS_RDS: &'static str = "/home/jka/probe/mock-rustdedicated";
    /// Absolute path of the `pgrep` executable.
    const EXEC_ABS_PGREP: &'static str = "/usr/bin/pgrep";
}
