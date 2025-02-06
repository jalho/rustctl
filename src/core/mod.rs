//! Core functionality of the program.

type Predicate = String;
type UnexpectedStatus = i32;
#[derive(Debug)]
pub enum Error {
    SystemError(crate::system::Error),
    SteamCMDError(Predicate, Option<UnexpectedStatus>, Option<std::io::Error>),
    InstallationInvalidFile(std::path::PathBuf, Option<std::io::Error>),
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::SystemError(err) => Some(err),
            Error::SteamCMDError(_, _, Some(err)) => Some(err),
            Error::SteamCMDError(_, _, None) => None,
            Error::InstallationInvalidFile(_, Some(err)) => Some(err),
            Error::InstallationInvalidFile(_, None) => None,
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SystemError(_) => write!(f, "system failure"),
            Error::SteamCMDError(predicate, Some(unexpected_status), _) => {
                write!(
                    f,
                    "SteamCMD failed with unexpected status {unexpected_status} to {predicate}"
                )
            }
            Error::SteamCMDError(predicate, None, _) => {
                write!(f, "SteamCMD failed without status to {predicate}")
            }
            Error::InstallationInvalidFile(path_buf, _) => write!(
                f,
                "invalid installation file: {}",
                path_buf.to_string_lossy()
            ),
        }
    }
}
impl From<crate::system::Error> for Error {
    fn from(value: crate::system::Error) -> Self {
        Self::SystemError(value)
    }
}

pub struct Game {
    /// Absolute path to the directory in which the game executable shall be
    /// installed.
    game_root_dir_absolute: &'static std::path::Path,
    /// Steam app ID of the game server.
    game_steam_app_id: u32,
    /// Filename (not the absolute path) of the game server executable.
    game_executable_filename: &'static std::path::Path,
    /// Filename (not the absolute path) of the game server manifest.
    game_manifest_filename: &'static std::path::Path,
    state: S,
}

impl Game {
    pub fn start(exclude_from_search: Option<std::path::PathBuf>) -> Result<Self, Error> {
        let game_root_dir_absolute: &'static std::path::Path = std::path::Path::new("/home/rust/");
        let game_steam_app_id: u32 = 258550;
        let game_executable_filename: &'static std::path::Path =
            std::path::Path::new("RustDedicated");
        let game_manifest_filename: &'static std::path::Path =
            std::path::Path::new("appmanifest_258550.acf");

        log::debug!("Determining initial state...");
        let state: S = determine_inital_state(
            game_executable_filename,
            game_steam_app_id,
            exclude_from_search,
        )?;
        log::debug!("Initial state determined: {state}");

        let game: Game = Self {
            state,
            game_root_dir_absolute,
            game_steam_app_id,
            game_executable_filename,
            game_manifest_filename,
        };
        let started: Game = game.transition(T::Start)?;
        return Ok(started);
    }

    fn transition(mut self, transition: T) -> Result<Self, Error> {
        match (&self.state, transition) {
            (S::I(_, RS::NR), T::_Install | T::_Stop) => Ok(self), // Nothing to do!

            (S::I(current, RS::NR), T::Start) => {
                let latest: SteamAppBuildId = Game::query_latest_version_info();
                if current.to != latest {
                    let updated: Updation = Game::update();
                    let pid: LinuxProcessId = Game::spawn();
                    self.state = S::I(updated, RS::R(pid));
                    return Ok(self);
                } else {
                    let pid: LinuxProcessId = Game::spawn();
                    self.state = S::I(current.clone(), RS::R(pid));
                    return Ok(self);
                }
            }

            (S::I(current, RS::NR), T::_Update) => {
                let latest: SteamAppBuildId = Game::query_latest_version_info();
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
                let latest: SteamAppBuildId = Game::query_latest_version_info();
                if current.to != latest {
                    Game::terminate(*pid);
                    let updated: Updation = Game::update();
                    let pid: LinuxProcessId = Game::spawn();
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
                let pid: LinuxProcessId = Game::spawn();
                self.state = S::I(installed, RS::R(pid));
                return Ok(self);
            }

            (S::NI, T::_Stop) => Ok(self), // Nothing to do!
        }
    }

    fn query_latest_version_info() -> SteamAppBuildId {
        todo!("query information of latest version of game server available using SteamCMD");
    }

    fn install(&self) -> Result<Updation, Error> {
        let argv: Vec<std::borrow::Cow<'_, str>> = vec![
            "+force_install_dir".into(),
            self.game_root_dir_absolute.to_string_lossy(),
            "+login".into(),
            "anonymous".into(),
            "+app_update".into(),
            self.game_steam_app_id.to_string().into(),
            "validate".into(),
            "+quit".into(),
        ];
        self.steamcmd_exec(argv)?;

        let mut path_executable: std::path::PathBuf = self.game_root_dir_absolute.to_path_buf();
        path_executable.push(self.game_executable_filename);

        let mut path_manifest: std::path::PathBuf = self.game_root_dir_absolute.to_path_buf();
        path_manifest.push(self.game_manifest_filename);

        let installation: Updation = Updation::read(&path_executable, &path_manifest)?;
        return Ok(installation);
    }

    fn update() -> Updation {
        todo!("update game server using SteamCMD");
    }

    fn spawn() -> LinuxProcessId {
        todo!("spawn game server process");
    }

    fn terminate(_pid: LinuxProcessId) {
        todo!("terminate game server process");
    }

    fn steamcmd_exec(&self, argv: Vec<std::borrow::Cow<'_, str>>) -> Result<(), Error> {
        let mut steamcmd: std::process::Command = std::process::Command::new("steamcmd");
        steamcmd.args(argv.iter().map(std::borrow::Cow::as_ref));

        if !self.game_root_dir_absolute.is_dir() {
            return Err(Error::SteamCMDError(
                format!(
                    "find working directory '{}'",
                    self.game_root_dir_absolute.to_string_lossy()
                ),
                None,
                None,
            ));
        }
        steamcmd.current_dir(self.game_root_dir_absolute);

        steamcmd.stdout(std::process::Stdio::piped());
        steamcmd.stderr(std::process::Stdio::piped());

        let output = match steamcmd.output() {
            Ok(n) => n,
            Err(err) => return Err(Error::SteamCMDError(String::from("spawn"), None, Some(err))),
        };
        if !output.status.success() {
            let predicate: String = argv.join(" ");
            return Err(Error::SteamCMDError(predicate, output.status.code(), None));
        }
        return Ok(());
    }
}
impl std::fmt::Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "");
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
                write!(f, "installed: Steam build ID {}, not running", updation.to)
            }
            S::I(updation, RS::R(pid)) => {
                write!(
                    f,
                    "installed: Steam build ID {}, running as PID {pid}",
                    updation.to
                )
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
    completed: chrono::DateTime<chrono::Utc>,
    /// Previous Steam build ID of the app. Can be `None` in the case of a
    /// fresh install.
    _from: Option<SteamAppBuildId>,
    /// Current Steam build ID of the app, i.e. the version to which the app
    /// was updated.
    to: SteamAppBuildId,
    /// Absolute path to the directory in which the app is installed.
    _root_dir_absolute: std::path::PathBuf,
    /// Name, _not the absolute path_, of the executable file.
    _executable_name: std::path::PathBuf,
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
            completed: last_modified,
            _from: None,
            to: build_id,
            _root_dir_absolute: root_dir_absolute,
            _manifest_name: manifest_name,
            _executable_name: executable_name,
        };

        return Ok(read);
    }
}
impl std::fmt::Display for Updation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Steam build ID {}, installation completed at {}",
            self.to, self.completed
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
    steam_app_id: u32,
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
        .join(format!("appmanifest_{steam_app_id}.acf")); // TODO: Remove duplicate definition of manifest file name
    let manifest: crate::system::ExistingFile =
        match crate::system::ExistingFile::check(&manifest_path) {
            Ok(n) => n,
            Err(_) => return Ok(S::NI),
        };

    let updation: Updation = Updation {
        completed: manifest.last_change,
        _from: None,
        to: crate::parsing::parse_buildid_from_manifest(&manifest.absolute_path_file)
            .expect("no build ID in manifest"),
        _root_dir_absolute: installed.absolute_path_parent,
        _executable_name: std::path::PathBuf::from(executable_name),
        _manifest_name: std::path::Path::new(&manifest.file_name.to_string_lossy().into_owned())
            .to_path_buf(),
    };

    let running: RS = match crate::system::check_process_running(executable_name)? {
        Some(pid) => RS::R(pid),
        None => RS::NR,
    };

    return Ok(S::I(updation, running));
}
