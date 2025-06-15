use crate::core::CrossTasksSharedState;
use proc::Dependency;
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
        log::debug!("Checking if SteamCMD or RustDedicated is already running...");
        let mut command = tokio::process::Command::new(Dependency::pgrep.get_absolute_path());
        let command = command.current_dir("/");
        let command = command.args(vec![Dependency::steamcmd.get_executable_name()]);
        let output = command.spawn().unwrap().wait_with_output().await.unwrap();
        log::debug!("\t{command:?}");
        log::debug!("\t{output:?}");
        if output.status.success() {
            // TODO: Implement non-recoverable error: Make error log and terminate the program!
            log::error!(
                "Cannot update and launch game: Dependency already running: {dependency}",
                dependency = Dependency::steamcmd
            );
        }

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

    #[allow(non_camel_case_types)]
    pub enum Dependency {
        pgrep,
        RustDedicated,
        steamcmd,
    }

    impl Dependency {
        pub fn get_absolute_path(&self) -> std::path::PathBuf {
            match self {
                Dependency::pgrep => std::path::Path::new(EXEC_ABS_PGREP).to_path_buf(),
                Dependency::RustDedicated => std::path::Path::new(EXEC_ABS_RDS).to_path_buf(),
                Dependency::steamcmd => std::path::Path::new(EXEC_ABS_STEAMCMD).to_path_buf(),
            }
        }

        pub fn get_executable_name(&self) -> String {
            let absolute_path: std::path::PathBuf = self.get_absolute_path();
            let executable_name: &std::ffi::OsStr = absolute_path.file_name().unwrap();
            return executable_name.to_string_lossy().to_owned().to_string();
        }
    }

    impl std::fmt::Display for Dependency {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "\"{}\" (\"{}\")",
                self.get_executable_name(),
                self.get_absolute_path().to_string_lossy(),
            )
        }
    }

    /// Absolute path of the `steamcmd` executable, i.e. the game server installer.
    const EXEC_ABS_STEAMCMD: &'static str = "/home/jka/probe/mock-steamcmd";
    /// Absolute path of the `RustDedicated` executable, i.e. the game server.
    const EXEC_ABS_RDS: &'static str = "/home/jka/probe/mock-rustdedicated";
    /// Absolute path of the `pgrep` executable.
    const EXEC_ABS_PGREP: &'static str = "/usr/bin/pgrep";
}
