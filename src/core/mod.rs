//! Core functionality of the program.

#[derive(Debug)]
pub enum Error {
    SystemError(crate::system::Error),
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::SystemError(err) => Some(err),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SystemError(_) => write!(f, "system failure"),
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
    pub fn start(exclude_from_search: Option<std::path::PathBuf>) -> Result<Self, Error> {
        log::debug!("Determining initial state...");
        let state: S = Game::determine_inital_state("RustDedicated", 258550, exclude_from_search)?;
        log::debug!("Initial state determined: {state}");
        let game: Game = Self { state };
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
                let installed: Updation = Game::install()?;
                self.state = S::I(installed, RS::NR);
                return Ok(self);
            }

            (S::NI, T::Start) => {
                let installed: Updation = Game::install()?;
                let pid: LinuxProcessId = Game::spawn();
                self.state = S::I(installed, RS::R(pid));
                return Ok(self);
            }

            (S::NI, T::_Stop) => Ok(self), // Nothing to do!
        }
    }

    fn determine_inital_state(
        executable_name: &'static str,
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
            .join(format!("appmanifest_{steam_app_id}.acf"));
        let manifest: crate::system::ExistingFile =
            match crate::system::ExistingFile::check(&manifest_path) {
                Ok(n) => n,
                Err(_) => return Ok(S::NI),
            };

        let updation: Updation = Updation {
            _completed: manifest.last_change,
            _from: None,
            to: crate::parsing::parse_buildid_from_manifest(&manifest.absolute_path_file)
                .expect("no build ID in manifest"),
            _root_dir_absolute: installed.absolute_path_parent,
            _executable_name: std::path::PathBuf::from(executable_name),
            _manifest_name: std::path::Path::new(
                &manifest.file_name.to_string_lossy().into_owned(),
            )
            .to_path_buf(),
        };

        let running: RS = match crate::system::check_process_running(executable_name)? {
            Some(pid) => RS::R(pid),
            None => RS::NR,
        };

        return Ok(S::I(updation, running));
    }

    fn query_latest_version_info() -> SteamAppBuildId {
        todo!("query information of latest version of game server available using SteamCMD");
    }

    fn install() -> Result<Updation, Error> {
        todo!("install game server using SteamCMD");
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
    _completed: chrono::DateTime<chrono::Utc>,
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

#[derive(Debug)]
/// Running state.
pub enum RS {
    /// Running.
    R(LinuxProcessId),
    /// Not running.
    NR,
}
