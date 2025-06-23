use crate::core::{
    CrossTasksSharedState,
    coroutines::Coroutine,
    error::{NonRecoverableError, format_error_source_tree},
};
use proc::DependencyLocated;
use rustctl_common::{
    snapshot::{Game, StateTransitionInitiator},
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
        let pgrep: DependencyLocated =
            match DependencyLocated::locate_installation(&decl_deps.pgrep).await {
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

        let steamcmd: DependencyLocated =
            match DependencyLocated::locate_installation(&decl_deps.steamcmd).await {
                Some(n) => n,
                None => todo!(),
            };
        let game_install_dir: &std::path::Path = self.get_install_dir();
        let install: tokio::process::Child = steamcmd
            .execute(
                game_install_dir,
                None,
                vec![
                    "+login".into(),
                    "anonymous".into(),
                    "+force_install_dir".into(),
                    game_install_dir.to_string_lossy().into(),
                    "+app_update".into(),
                    self.steam_app_id.to_string(),
                    "validate".into(),
                    "+quit".into(),
                ],
            )
            .await;
        let output: std::process::Output = install.wait_with_output().await.unwrap();
        log::debug!("TODO: {output:?}");

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

#[derive(Clone)]
pub struct GameState {
    /// Absolute path to the game server executable named `RustDedicated`
    /// expected to be installed with SteamCMD (`steamcmd`).
    expected_installation_path_absolute: std::path::PathBuf,

    /// Application ID of the Rust game server (`RustDedicated`) in Steam.
    steam_app_id: u32,

    pub last_state_transition_at: chrono::DateTime<chrono::Utc>,
    pub last_state_transition_inititated_by: StateTransitionInitiator,
    pub game: Game,
}

impl GameState {
    pub fn init() -> Result<Self, NonRecoverableError> {
        let expected_installation_path_absolute: std::path::PathBuf =
            std::path::Path::new("/home/rust/RustDedicated").to_path_buf();
        let dir: &std::path::Path = expected_installation_path_absolute.parent().unwrap();
        if !dir.is_dir() {
            let err = NonRecoverableError::MissingWorkDirGame {
                expected_dir_path_abs: dir.to_path_buf(),
            };
            log::error!(
                "Non-recoverable error: {source_tree}",
                source_tree = format_error_source_tree(&err)
            );
            return Err(err);
        }

        Ok(Self {
            expected_installation_path_absolute,
            steam_app_id: 258550,

            last_state_transition_at: chrono::Utc::now(),
            last_state_transition_inititated_by: StateTransitionInitiator::AutomaticBySytem,
            game: Game::Init(rustctl_common::state_machine::Init {}),
        })
    }

    fn get_install_dir(&self) -> &std::path::Path {
        self.expected_installation_path_absolute.parent().unwrap()
    }
}

pub mod proc {
    use std::{
        collections::HashMap,
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
    pub struct DependencyLocated {
        pub executable_path_absolute: std::path::PathBuf,
    }

    impl DependencyLocated {
        pub async fn locate_installation(declared: &DependencyDeclared) -> Option<Self> {
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

        pub async fn execute(
            &self,
            work_dir: &std::path::Path,
            env: Option<HashMap<String, String>>,
            argv: Vec<String>,
        ) -> tokio::process::Child {
            let mut cmd = tokio::process::Command::new(&self.executable_path_absolute);
            cmd.args(argv);
            cmd.current_dir(work_dir);
            if let Some(env) = env {
                cmd.envs(env);
            }
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
            let child_process: tokio::process::Child = cmd.spawn().unwrap();
            return child_process;
        }

        pub fn get_executable_name(&self) -> String {
            self.executable_path_absolute
                .to_owned()
                .to_string_lossy()
                .to_string()
        }
    }

    impl Display for DependencyLocated {
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
        pgrep: &DependencyLocated,
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
