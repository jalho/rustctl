use crate::core::CrossTasksSharedState;
use rustctl_common::{
    snapshot::Game,
    state_machine::{NotRunning, ShutdownInProgress, StartupInProgress},
};
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
    /// Update and launch game server.
    async fn update_and_launch(&mut self);

    /// Make a client message driven state transition in the game state.
    async fn handle_client_message(&mut self, client_msg: String);
}

impl GameStateMachine for Game {
    async fn update_and_launch(&mut self) {
        log::debug!("TODO: Check if SteamCMD or RustDedicated is already running");
        log::debug!("TODO: Install or update RustDedicatedd using SteamCMD");
        log::debug!("TODO: Launch RustDedicated");
        *self = Game::NotRunning(NotRunning {
            state_transitioned_into_at: chrono::Utc::now(),
        });
    }

    async fn handle_client_message(&mut self, client_msg: String) {
        /*
         * TODO: Take into consideration:
         *
         * - Check if a command is even expected at this time: Check if the
         *   received command matches the current state
         *
         * - Extract args from commanding client message if necessary
         *
         * - Make the state transition: Mutate self
         */
        match self {
            Game::Init(_state) => {
                /*
                 * Nothing to do: Transition from Init should happen
                 * automatically, and not per client message.
                 */
            }
            Game::NotRunning(state) => {
                log::debug!(
                    "TODO: Launch game with args: '{client_msg}' -- Current state: {state:?}"
                );
                *self = Game::StartupInProgress(StartupInProgress {
                    state_transitioned_into_at: chrono::Utc::now(),
                });
            }
            Game::StartupInProgress(state) => {
                log::debug!(
                    "TODO: Abort game startup with args: '{client_msg}' -- Current state: {state:?}"
                );
                *self = Game::NotRunning(NotRunning {
                    state_transitioned_into_at: chrono::Utc::now(),
                });
            }
            Game::RunningHealthy(state) => {
                log::debug!(
                    "TODO: Save game state and close with args: '{client_msg}' -- Current state: {state:?}"
                );
                *self = Game::ShutdownInProgress(ShutdownInProgress {
                    state_transitioned_into_at: chrono::Utc::now(),
                });
            }
            Game::ShutdownInProgress(_state) => {
                /*
                 * Nothing to do: Initiated game shutdown sequence cannot be
                 * canceled.
                 */
            }
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
