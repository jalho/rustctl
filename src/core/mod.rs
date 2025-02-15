//! Core functionality of the program.

type Predicate = String;
type Object = String;
type UnexpectedStatus = i32;
#[derive(Debug)]
pub enum Error {
    SystemError(crate::system::Error),
    /// Failure while executing SteamCMD: Failed to spawn process, unexpected
    /// termination status, unusable working directory etc.
    SteamCMDExecError(Predicate, Option<UnexpectedStatus>, Option<std::io::Error>),
    /// The output of an executed SteamCMD command has some unexpected
    /// characteristic.
    SteamCMDUnexpectedOutput(Predicate, Object),
    InstallationInvalidFile(std::path::PathBuf, Option<std::io::Error>),
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::SystemError(err) => Some(err),
            Error::SteamCMDExecError(_, _, Some(err)) => Some(err),
            Error::SteamCMDExecError(_, _, None) => None,
            Error::InstallationInvalidFile(_, Some(err)) => Some(err),
            Error::InstallationInvalidFile(_, None) => None,
            Error::SteamCMDUnexpectedOutput(_, _) => None,
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SystemError(_) => write!(f, "system failure"),
            Error::SteamCMDExecError(predicate, Some(unexpected_status), _) => {
                write!(
                    f,
                    "SteamCMD failed with unexpected status {unexpected_status} to {predicate}"
                )
            }
            Error::SteamCMDExecError(predicate, None, _) => {
                write!(f, "SteamCMD failed without status to {predicate}")
            }
            Error::InstallationInvalidFile(path_buf, _) => write!(
                f,
                "invalid installation file: {}",
                path_buf.to_string_lossy()
            ),
            Error::SteamCMDUnexpectedOutput(predicate, object) => {
                write!(
                    f,
                    "unexpected output from SteamCMD to {predicate}: {object}"
                )
            }
        }
    }
}
impl From<crate::system::Error> for Error {
    fn from(value: crate::system::Error) -> Self {
        Self::SystemError(value)
    }
}

pub struct Game {
    state: S,
}

impl Game {
    /// Absolute path to the directory in which the game executable shall be
    /// installed.
    fn get_game_root_dir_absolute() -> &'static std::path::Path {
        std::path::Path::new("/home/rust/")
    }

    /// Steam app ID of the game server.
    fn get_game_steam_app_id() -> u32 {
        258550
    }

    /// Filename (not the absolute path) of the game server executable.
    fn get_game_executable_filename() -> &'static std::path::Path {
        std::path::Path::new("RustDedicated")
    }

    /// Filename (not the absolute path) of the game server manifest.
    fn get_game_manifest_filename() -> &'static std::path::Path {
        std::path::Path::new("appmanifest_258550.acf")
    }

    pub fn start(exclude_from_search: Option<std::path::PathBuf>) -> Result<Self, Error> {
        log::debug!("Determining initial state...");
        let state: S =
            determine_inital_state(Game::get_game_executable_filename(), exclude_from_search)?;
        log::info!("Initial state determined: {state}");

        let game: Game = Self { state };
        let started: Game = game.transition(T::Start)?;
        return Ok(started);
    }

    fn transition(mut self, transition: T) -> Result<Self, Error> {
        match (&self.state, transition) {
            (S::I(_, RS::NR), T::_Install | T::_Stop) => Ok(self), // Nothing to do!

            (S::I(current, RS::NR), T::Start) => {
                log::debug!("Querying latest available version from remote...");
                let latest: SteamAppBuildId = self.query_latest_version_info()?;
                if current.to != latest {
                    log::info!(
                        "There is an update available: Steam app build ID from {} to {}",
                        current.to,
                        latest
                    );
                    log::info!("Updating the game installation...");
                    let updated: Updation = Game::update();
                    log::info!("Updated the game from {} to {}", updated.from, updated.to);

                    log::info!("Spawning game process...");
                    let pid: LinuxProcessId = self.spawn(
                        &updated.root_dir_absolute,
                        &updated.root_dir_absolute.join(&updated.executable_name),
                    );
                    self.state = S::I(updated, RS::R(pid));
                    return Ok(self);
                } else {
                    log::info!(
                        "Current installation is up to date: Steam app build ID {}",
                        current.to
                    );

                    log::info!("Spawning game process...");
                    let pid: LinuxProcessId = self.spawn(
                        &current.root_dir_absolute,
                        &current.root_dir_absolute.join(&current.executable_name),
                    );
                    self.state = S::I(current.clone(), RS::R(pid));
                    return Ok(self);
                }
            }

            (S::I(current, RS::NR), T::_Update) => {
                let latest: SteamAppBuildId = self.query_latest_version_info()?;
                if current.to != latest {
                    let updated: Updation = Game::update();
                    self.state = S::I(updated, RS::NR);
                    return Ok(self);
                } else {
                    return Ok(self);
                }
            }

            (S::I(_, RS::R(_)), T::_Install | T::Start) => Ok(self), // Nothing to do!

            (S::I(current, RS::R(pid)), T::_Stop) => {
                Game::terminate(*pid);
                self.state = S::I(current.clone(), RS::NR);
                return Ok(self);
            }

            (S::I(current, RS::R(pid)), T::_Update) => {
                let latest: SteamAppBuildId = self.query_latest_version_info()?;
                if current.to != latest {
                    Game::terminate(*pid);
                    let updated: Updation = Game::update();
                    let pid: LinuxProcessId = self.spawn(
                        &updated.root_dir_absolute,
                        &updated.root_dir_absolute.join(&updated.executable_name),
                    );
                    self.state = S::I(updated, RS::R(pid));
                    return Ok(self);
                } else {
                    return Ok(self);
                }
            }

            (S::NI, T::_Install | T::_Update) => {
                log::debug!("Installing game...");
                let installed: Updation = self.install()?;
                log::info!("Installed game: {installed}");
                self.state = S::I(installed, RS::NR);
                return Ok(self);
            }

            (S::NI, T::Start) => {
                log::debug!("Installing game...");
                let installed: Updation = self.install()?;
                log::info!("Installed game: {installed}");
                let pid: LinuxProcessId = self.spawn(
                    &installed.root_dir_absolute,
                    &installed.root_dir_absolute.join(&installed.executable_name),
                );
                self.state = S::I(installed, RS::R(pid));
                return Ok(self);
            }

            (S::NI, T::_Stop) => Ok(self), // Nothing to do!
        }
    }

    fn query_latest_version_info(&self) -> Result<SteamAppBuildId, Error> {
        /* Unsure if the "app_info_update" step is necessary or whether the
        following "app_info_print" alone is sufficient to get latest information
        from the remote... */
        let argv: Vec<std::borrow::Cow<'_, str>> =
            vec!["+app_info_update".into(), "1".into(), "+quit".into()];
        self.steamcmd_exec(argv)?;

        let argv: Vec<std::borrow::Cow<'_, str>> = vec![
            "+app_info_print".into(),
            Game::get_game_steam_app_id().to_string().into(),
            "+quit".into(),
        ];
        let stdout_utf8: String = self.steamcmd_exec(argv)?;
        let build_id: u32 = match crate::parsing::parse_buildid_from_buffer(&stdout_utf8) {
            Some(n) => n,
            None => {
                return Err(Error::SteamCMDUnexpectedOutput(
                    String::from("parse build ID"),
                    stdout_utf8,
                ));
            }
        };
        return Ok(build_id);
    }

    fn install(&self) -> Result<Updation, Error> {
        let argv: Vec<std::borrow::Cow<'_, str>> = vec![
            "+force_install_dir".into(),
            Game::get_game_root_dir_absolute().to_string_lossy(),
            "+login".into(),
            "anonymous".into(),
            "+app_update".into(),
            Game::get_game_steam_app_id().to_string().into(),
            "validate".into(),
            "+quit".into(),
        ];
        self.steamcmd_exec(argv)?;

        let mut path_executable: std::path::PathBuf =
            Game::get_game_root_dir_absolute().to_path_buf();
        path_executable.push(Game::get_game_executable_filename());

        let mut path_manifest: std::path::PathBuf =
            Game::get_game_root_dir_absolute().to_path_buf();
        path_manifest.push("steamapps");
        path_manifest.push(Game::get_game_manifest_filename());

        let installation: Updation = Updation::read(&path_executable, &path_manifest)?;
        return Ok(installation);
    }

    fn update() -> Updation {
        todo!("update game server using SteamCMD");
    }

    // TODO: Define parameter driving data state: Should some or all of the
    //       data of the program be removed before spawning the process? (Namely
    //       previous game world maps, player blueprints and any other game
    //       data...)
    fn spawn(&self, work_dir: &std::path::Path, executable: &std::path::Path) -> LinuxProcessId {
        let mut cmd_rds = std::process::Command::new(executable);
        // TODO: Define LD_LIBRARY_PATH env var (or something like that, if necessary?)
        cmd_rds.current_dir(work_dir);
        let argv: Vec<&str> = vec![
            // TODO: Get world seed and size as args and further from some database?
        ];
        cmd_rds.args(&argv);
        cmd_rds.stdout(std::process::Stdio::piped());
        cmd_rds.stderr(std::process::Stdio::piped());

        let mut child = match cmd_rds.spawn() {
            Ok(n) => n,
            Err(_) => todo!("define error case"),
        };
        let pid: LinuxProcessId = child.id();
        log::info!("Game server process spawned as PID {pid}");
        let (th_stdout, th_stderr) = match crate::system::trace_log_child_output(&mut child) {
            Ok(n) => n,
            Err(_) => todo!("define error case"),
        };

        // TODO: Return the STDOUT, STDERR thread join handles, and don't wait for them to terminate here
        _ = th_stdout.join();
        _ = th_stderr.join();
        return pid;
    }

    fn terminate(_pid: LinuxProcessId) {
        todo!("terminate game server process");
    }

    fn steamcmd_exec(&self, argv: Vec<std::borrow::Cow<'_, str>>) -> Result<String, Error> {
        let steamcmd_executable: &'static str = "steamcmd";
        let mut steamcmd: std::process::Command = std::process::Command::new(steamcmd_executable);
        steamcmd.args(argv.iter().map(std::borrow::Cow::as_ref));

        if !Game::get_game_root_dir_absolute().is_dir() {
            return Err(Error::SteamCMDExecError(
                format!(
                    "find working directory '{}'",
                    Game::get_game_root_dir_absolute().to_string_lossy()
                ),
                None,
                None,
            ));
        }
        steamcmd.current_dir(Game::get_game_root_dir_absolute());

        steamcmd.stdout(std::process::Stdio::piped());
        steamcmd.stderr(std::process::Stdio::piped());

        log::trace!("{steamcmd_executable} {}", argv.join(" "));
        let child: std::process::Child = match steamcmd.spawn() {
            Ok(n) => n,
            Err(err) => {
                return Err(Error::SteamCMDExecError(
                    String::from("spawn"),
                    None,
                    Some(err),
                ))
            }
        };

        let (stdout, _stderr, exit_status) =
            match crate::system::trace_log_child_output_and_wait_to_terminate(child) {
                Ok(n) => n,
                Err(err) => {
                    return Err(Error::SteamCMDExecError(
                        String::from("terminate"),
                        None,
                        Some(err),
                    ))
                }
            };

        if !exit_status.success() {
            let predicate: String = argv.join(" ");
            return Err(Error::SteamCMDExecError(
                predicate,
                exit_status.code(),
                None,
            ));
        }

        return Ok(stdout);
    }
}

/// State of the machine.
#[derive(Debug)]
enum S {
    /// Not installed.
    NI,
    /// Installed.
    I(Updation, RS),
}
impl std::fmt::Display for S {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            S::NI => write!(f, "not installed"),
            S::I(updation, RS::NR) => {
                write!(f, "installed: {updation}, not running")
            }
            S::I(updation, RS::R(pid)) => {
                write!(f, "installed: {updation}, running as PID {pid}")
            }
        }
    }
}

#[derive(Debug)]
/// Transition of the state machine.
pub enum T {
    _Install,
    Start,
    _Stop,
    _Update,
}

pub type SteamAppBuildId = u32;

pub type LinuxProcessId = u32;

/// Represents a fresh installation or _updation_ (:D) of an existing
/// installation.
#[derive(Debug, Clone)]
struct Updation {
    /// Timestamp of when the app's current version was installed.
    installed_at: chrono::DateTime<chrono::Utc>,
    /// Previous Steam build ID of the app. The value can be the same as the
    /// _current_ (alias _to_) if there is no _previous_ value in the context
    /// of evaluation, like in the case of a fresh installation as opposed to
    /// updating an existing installation.
    from: SteamAppBuildId,
    /// Current Steam build ID of the app, i.e. the version to which the app
    /// was updated.
    to: SteamAppBuildId,
    /// Absolute path to the directory in which the app is installed.
    root_dir_absolute: std::path::PathBuf,
    /// Name, _not the absolute path_, of the executable file.
    executable_name: std::path::PathBuf,
    /// Name, _not the absolute path_, of the Steam app's manifest file.
    _manifest_name: std::path::PathBuf,
}
impl Updation {
    /// Try to read given files to determine some metadata of a now existing
    /// installation.
    pub fn read(
        game_executable_path: &std::path::Path,
        manifest_path: &std::path::Path,
    ) -> Result<Self, Error> {
        let metadata: std::fs::Metadata = match manifest_path.metadata() {
            Ok(n) => n,
            Err(err) => {
                return Err(Error::InstallationInvalidFile(
                    manifest_path.to_path_buf(),
                    Some(err),
                ))
            }
        };
        let last_modified: chrono::DateTime<chrono::Utc> = match chrono::DateTime::from_timestamp(
            std::os::unix::fs::MetadataExt::mtime(&metadata),
            0,
        ) {
            Some(n) => n,
            None => {
                return Err(Error::InstallationInvalidFile(
                    manifest_path.to_path_buf(),
                    None,
                ))
            }
        };
        let build_id: u32 = match crate::parsing::parse_buildid_from_manifest(manifest_path) {
            Some(n) => n,
            None => {
                return Err(Error::InstallationInvalidFile(
                    manifest_path.to_path_buf(),
                    None,
                ))
            }
        };

        let root_dir_absolute: std::path::PathBuf = match game_executable_path.canonicalize() {
            Ok(n) => n,
            Err(err) => {
                return Err(Error::InstallationInvalidFile(
                    game_executable_path.to_path_buf(),
                    Some(err),
                ))
            }
        };

        let executable_name: std::path::PathBuf = match game_executable_path.file_name() {
            Some(n) => std::path::PathBuf::from(n),
            None => {
                return Err(Error::InstallationInvalidFile(
                    game_executable_path.to_path_buf(),
                    None,
                ))
            }
        };

        let manifest_name: std::path::PathBuf = match manifest_path.file_name() {
            Some(n) => std::path::PathBuf::from(n),
            None => {
                return Err(Error::InstallationInvalidFile(
                    manifest_path.to_path_buf(),
                    None,
                ))
            }
        };

        let read: Updation = Self {
            installed_at: last_modified,
            from: build_id,
            to: build_id,
            root_dir_absolute,
            _manifest_name: manifest_name,
            executable_name,
        };

        return Ok(read);
    }
}
impl std::fmt::Display for Updation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Steam build ID {}, installed at {}",
            self.to, self.installed_at
        )
    }
}

#[derive(Debug)]
/// Running state.
pub enum RS {
    /// Running.
    R(LinuxProcessId),
    /// Not running.
    NR,
}

fn determine_inital_state(
    executable_name: &'static std::path::Path,
    exclude_from_search: Option<std::path::PathBuf>,
) -> Result<S, crate::system::Error> {
    let installed: crate::system::ExistingFile =
        match crate::system::find_single_file(executable_name, exclude_from_search) {
            Ok(Some(n)) => n,
            Ok(None) => return Ok(S::NI),
            Err(crate::system::Error::FileNotFound(_)) => return Ok(S::NI),
            Err(err) => return Err(err),
        };

    let manifest_path: std::path::PathBuf = installed
        .absolute_path_parent
        .join("steamapps")
        .join(Game::get_game_manifest_filename());
    let manifest: crate::system::ExistingFile =
        match crate::system::ExistingFile::check(&manifest_path) {
            Ok(n) => n,
            Err(_) => return Ok(S::NI),
        };

    let build_id: u32 =
        match crate::parsing::parse_buildid_from_manifest(&manifest.absolute_path_file) {
            Some(n) => n,
            None => todo!("define error case"),
        };

    let updation: Updation = Updation {
        installed_at: manifest.last_change,
        from: build_id,
        to: build_id,
        root_dir_absolute: installed.absolute_path_parent,
        executable_name: std::path::PathBuf::from(executable_name),
        _manifest_name: std::path::Path::new(&manifest.file_name.to_string_lossy().into_owned())
            .to_path_buf(),
    };

    let running: RS = match crate::system::check_process_running(executable_name)? {
        Some(pid) => RS::R(pid),
        None => RS::NR,
    };

    return Ok(S::I(updation, running));
}
