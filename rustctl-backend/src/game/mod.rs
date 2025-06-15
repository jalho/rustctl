use crate::core::{
    CrossTasksSharedState,
    error::{NonRecoverableError, format_error_source_tree},
};
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
    _shutdown_tx: tokio::sync::mpsc::Sender<()>,
    interval: Duration,
    _shared: Arc<Mutex<CrossTasksSharedState>>,
) -> Result<(), NonRecoverableError> {
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
    Ok(())
}

pub trait GameStateMachine {
    /// Update and launch game server.
    async fn update_and_launch(
        &mut self,
        initiator: StateTransitionInitiator,
    ) -> Result<(), NonRecoverableError>;

    /// Make a client message driven state transition in the game state.
    async fn handle_client_message(
        &mut self,
        client_msg: String,
        initiator: StateTransitionInitiator,
    );
}

impl GameStateMachine for GameState {
    async fn update_and_launch(
        &mut self,
        initiator: StateTransitionInitiator,
    ) -> Result<(), NonRecoverableError> {
        if let Some(pid) = proc::is_running(Dependency::steamcmd).await {
            let err = NonRecoverableError::ConcurrentDependency {
                cannot_display: String::from("cannot update and launch game"),
                dependency: Dependency::steamcmd,
                pid,
            };
            log::error!(
                "Non-recoverable error: {source_tree}",
                source_tree = format_error_source_tree(&err)
            );
            return Err(err);
        }

        if let Some(pid) = proc::is_running(Dependency::RustDedicated).await {
            let err = NonRecoverableError::ConcurrentDependency {
                cannot_display: String::from("cannot update and launch game"),
                dependency: Dependency::RustDedicated,
                pid,
            };
            log::error!(
                "Non-recoverable error: {source_tree}",
                source_tree = format_error_source_tree(&err)
            );
            return Err(err);
        }

        log::debug!("TODO: Install or update RustDedicated using SteamCMD");

        log::debug!("TODO: Launch RustDedicated");

        // TODO: Mutate self by assigning new state to self.game once game startup is in progress!
        self.game = Game::NotRunning(NotRunning {});
        self.last_state_transition_at = chrono::Utc::now();
        self.last_state_transition_inititated_by = initiator;
        Ok(())
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

pub mod proc {
    use std::process::Stdio;

    #[derive(Debug)]
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
            executable_name.to_string_lossy().into_owned()
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
    const EXEC_ABS_STEAMCMD: &str = "/home/jka/probe/mock-steamcmd";
    /// Absolute path of the `RustDedicated` executable, i.e. the game server.
    const EXEC_ABS_RDS: &str = "/home/jka/probe/mock-rustdedicated";
    /// Absolute path of the `pgrep` executable.
    const EXEC_ABS_PGREP: &str = "/usr/bin/pgrep";

    /// Returns the PID of the running dependency, if it's running.
    pub async fn is_running(dependency: Dependency) -> Option<u32> {
        let mut command = tokio::process::Command::new(Dependency::pgrep.get_absolute_path());
        let command = command.current_dir("/");
        let command = command.args(vec![dependency.get_executable_name()]);
        let command = command.stdout(Stdio::piped());
        let command = command.stderr(Stdio::piped());
        let output = command.spawn().unwrap().wait_with_output().await.unwrap();
        if output.status.success() {
            let stdout: String = String::from_utf8(output.stdout).unwrap();
            let stdout = stdout.trim();
            let pid: u32 = stdout.parse().unwrap();
            Some(pid)
        } else {
            None
        }
    }
}
