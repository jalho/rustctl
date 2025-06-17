use crate::core::{
    CrossTasksSharedState,
    coroutines::Coroutine,
    error::{NonRecoverableError, format_error_source_tree},
};
use proc::DependencyChecked;
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
        dependencies: &proc::DependenciesDeclared,
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
        decl_deps: &proc::DependenciesDeclared,
    ) -> Result<(), NonRecoverableError> {
        let pgrep: DependencyChecked = match DependencyChecked::check(&decl_deps.pgrep).await {
            Some(n) => n,
            None => todo!(),
        };

        if let Some(pid) = proc::is_running(&pgrep, &decl_deps.steamcmd).await {
            let err = NonRecoverableError::ConcurrentDependency {
                dependency: decl_deps.steamcmd.clone(),
                pid,
            };
            log::error!(
                "Non-recoverable error: {source_tree}",
                source_tree = format_error_source_tree(&err)
            );
            return Err(err);
        }

        if let Some(pid) = proc::is_running(&pgrep, &decl_deps.RustDedicated).await {
            let err = NonRecoverableError::ConcurrentDependency {
                dependency: decl_deps.RustDedicated.clone(),
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
    use std::{
        fmt::Display,
        path::{Path, PathBuf},
        process::Stdio,
    };
    use tokio::fs;

    #[allow(non_snake_case)]
    pub struct DependenciesDeclared {
        /// A common Linux utility.
        pub pgrep: DependencyDeclared,

        /// The game server installer.
        pub steamcmd: DependencyDeclared,

        /// The game server executable. May be `None` if not yet installed.
        pub RustDedicated: DependencyDeclared,
    }

    #[derive(Clone, Debug)]
    pub struct DependencyDeclared {
        pub expected_executable_name: String,
    }

    impl DependencyDeclared {
        pub fn declare(expected_executable_name: &str) -> Self {
            Self {
                expected_executable_name: expected_executable_name.to_owned(),
            }
        }
    }

    impl Display for DependencyDeclared {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.expected_executable_name)
        }
    }

    #[derive(Clone, Debug)]
    pub struct DependencyChecked {
        pub executable_path_absolute: std::path::PathBuf,
    }

    impl DependencyChecked {
        pub async fn check(declared: &DependencyDeclared) -> Option<Self> {
            let executable_name = &declared.expected_executable_name;
            let path = Path::new(executable_name);
            if path.components().count() > 1 {
                if fs::metadata(path).await.ok()?.is_file() {
                    return Some(Self {
                        executable_path_absolute: path.canonicalize().ok()?,
                    });
                } else {
                    return None;
                }
            }

            let raw_path = std::env::var_os("PATH")?;
            let path_str = raw_path.to_string_lossy();

            let dirs = if path_str.contains(':') {
                std::env::split_paths(&raw_path).collect::<Vec<_>>()
            } else {
                path_str
                    .split_whitespace()
                    .map(PathBuf::from)
                    .collect::<Vec<_>>()
            };

            'paths: for dir in dirs {
                let full_path = dir.join(executable_name);
                let found_path = match fs::metadata(&full_path).await {
                    Ok(n) => n,
                    Err(_) => continue 'paths,
                };
                if found_path.is_file() {
                    return Some(Self {
                        executable_path_absolute: full_path,
                    });
                } else {
                    continue 'paths;
                }
            }
            None
        }

        pub fn get_executable_name(&self) -> String {
            self.executable_path_absolute
                .to_owned()
                .to_string_lossy()
                .to_string()
        }
    }

    impl Display for DependencyChecked {
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
    pub async fn is_running(
        pgrep: &DependencyChecked,
        dependency: &DependencyDeclared,
    ) -> Option<u32> {
        let mut command = tokio::process::Command::new(&pgrep.executable_path_absolute);
        let command = command.current_dir("/");
        let command = command.args(vec![&dependency.expected_executable_name]);
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
