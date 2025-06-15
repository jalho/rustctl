use crate::core::CrossTasksSharedState;
use rustctl_common::{
    snapshot::{Game, GameState, StateTransitionInitiator},
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
    async fn update_and_launch(&mut self, initiator: StateTransitionInitiator);

    /// Make a client message driven state transition in the game state.
    async fn handle_client_message(
        &mut self,
        client_msg: String,
        initiator: StateTransitionInitiator,
    );
}

impl GameStateMachine for GameState {
    async fn update_and_launch(&mut self, initiator: StateTransitionInitiator) {
        log::debug!("TODO: Check if SteamCMD or RustDedicated is already running");
        log::debug!("TODO: Install or update RustDedicatedd using SteamCMD");
        log::debug!("TODO: Launch RustDedicated");
        self.game = Game::NotRunning(NotRunning {});
        self.last_state_transition_at = chrono::Utc::now();
        self.last_state_transition_inititated_by = initiator;
    }

    async fn handle_client_message(
        &mut self,
        client_msg: String,
        initiator: StateTransitionInitiator,
    ) {
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
        match self.game {
            Game::Init(ref _state) => {
                /*
                 * Nothing to do: Transition from Init should happen
                 * automatically, and not per client message.
                 */
            }
            Game::NotRunning(ref state) => {
                log::debug!(
                    "TODO: Launch game with args: '{client_msg}' -- Current state: {state:?}"
                );
                self.game = Game::StartupInProgress(StartupInProgress {});
                self.last_state_transition_at = chrono::Utc::now();
                self.last_state_transition_inititated_by = initiator;
            }
            Game::StartupInProgress(ref state) => {
                log::debug!(
                    "TODO: Abort game startup with args: '{client_msg}' -- Current state: {state:?}"
                );
                self.game = Game::NotRunning(NotRunning {});
                self.last_state_transition_at = chrono::Utc::now();
                self.last_state_transition_inititated_by = initiator;
            }
            Game::RunningHealthy(ref state) => {
                log::debug!(
                    "TODO: Save game state and close with args: '{client_msg}' -- Current state: {state:?}"
                );
                self.game = Game::ShutdownInProgress(ShutdownInProgress {});
                self.last_state_transition_at = chrono::Utc::now();
                self.last_state_transition_inititated_by = initiator;
            }
            Game::ShutdownInProgress(ref _state) => {
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
