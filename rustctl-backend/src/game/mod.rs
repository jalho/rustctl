use crate::core::{
    CrossTasksSharedState,
    coroutines::Coroutine,
    error::{NonRecoverableError, format_error_source_tree},
};
use rustctl_common::{
    snapshot::{Game, GameState, StateTransitionInitiator},
    state_machine::{NotRunning, ShutdownInProgress, StartupInProgress},
};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub async fn read_state(
    coroutine_identity: Coroutine,
    cancel: CancellationToken,
    _shutdown_tx: tokio::sync::mpsc::Sender<Coroutine>,
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
    log::info!("Coroutine done: {coroutine_identity}");
    Ok(())
}

pub trait GameStateMachine {
    /// Update and launch game server.
    async fn update_and_launch(
        &mut self,
        initiator: StateTransitionInitiator,
        dependencies: &proc::Dependencies,
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
        dependencies: &proc::Dependencies,
    ) -> Result<(), NonRecoverableError> {
        if let Some(pid) = proc::is_running(dependencies, &dependencies.steamcmd).await {
            let err = NonRecoverableError::ConcurrentDependency {
                cannot_display: String::from("cannot update and launch game"),
                dependency: dependencies.steamcmd.clone(),
                pid,
            };
            log::error!(
                "Non-recoverable error: {source_tree}",
                source_tree = format_error_source_tree(&err)
            );
            return Err(err);
        }

        if let Some(pid) = proc::is_running(dependencies, &dependencies.RustDedicated).await {
            let err = NonRecoverableError::ConcurrentDependency {
                cannot_display: String::from("cannot update and launch game"),
                dependency: dependencies.RustDedicated.clone(),
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
    use std::{fmt::Display, process::Stdio};

    use crate::core::error::NonRecoverableError;

    #[allow(non_snake_case)]
    pub struct Dependencies {
        pub pgrep: Dependency,
        pub steamcmd: Dependency,
        pub RustDedicated: Dependency,
    }

    impl Dependencies {
        pub async fn check() -> Result<Self, NonRecoverableError> {
            todo!("check if the dependencies exist");
        }
    }

    #[derive(Clone, Debug)]
    pub struct Dependency {
        pub executable_path_absolute: std::path::PathBuf,
    }

    impl Dependency {
        pub fn get_executable_name(&self) -> String {
            self.executable_path_absolute
                .to_owned()
                .to_string_lossy()
                .to_string()
        }
    }

    impl Display for Dependency {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                r#""{name}" ("{absolute_path}")"#,
                name = self.get_executable_name(),
                absolute_path = self.executable_path_absolute.to_string_lossy(),
            )
        }
    }

    /// Returns the PID of the running dependency, if it's running.
    pub async fn is_running(dependencies: &Dependencies, dependency: &Dependency) -> Option<u32> {
        let mut command =
            tokio::process::Command::new(&dependencies.pgrep.executable_path_absolute);
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
