//! Core functionality of the program.

enum Error {
    UndecideableInstallationStatus(crate::system::FindSingleFileError),
    CannotCheckUpdates(CCU),
    FailedInstallAttempt(FIA),
    GameStartError {
        system_error: std::io::Error,
        executable_path_absolute: std::path::PathBuf,
        exec_dir_path_absolute: std::path::PathBuf,
    },
}

/// CannotCheckUpdates
enum CCU {
    AmbiguousLocalCache {
        cache_filename_seeked: std::path::PathBuf,
        cache_paths_absolute_found: Vec<std::path::PathBuf>,
    },
    CannotWipeLocalCache {
        cache_path_absolute_found: std::path::PathBuf,
        system_error: std::io::Error,
    },
    CannotFetchRemoteInfo(SteamCMDErrorMeta),
    MalformedSteamAppInfo(MalformedSteamAppInfo),
}

/// FailedInstallAttempt
enum FIA {
    CannotInstall(SteamCMDErrorMeta),
    InvalidInstallation(II),
}

/// InvalidInstallation
enum II {
    MissingRequiredFile {
        filename_seeked: std::path::PathBuf,
    },
    AmbiguousRequiredFile {
        paths_absolute_found: Vec<std::path::PathBuf>,
    },
}

enum MalformedSteamAppInfo {
    UnexpectedFormat { data: Vec<u8> },
    MissingPublicBranch { data: Vec<u8> },
    AmbiguousPublicBranch { data: Vec<u8> },
    InvalidPublicBranchValue { data: Vec<u8> },
}

struct SteamCMDErrorMeta {
    steamcmd_command_argv: Vec<std::borrow::Cow<'static, str>>,
    steamcmd_exit_status: std::process::ExitStatus,
    steamcmd_stdout: Vec<u8>,
    steamcmd_stderr: Vec<u8>,
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
            match determine_inital_state(Game::get_game_executable_filename(), exclude_from_search)
            {
                Ok(n) => n,
                Err(err) => return Err(Error::UndecideableInstallationStatus(err)),
            };
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
                    let running: RunningGameServerProcess = self.spawn(
                        &updated.root_dir_absolute,
                        &updated.root_dir_absolute.join(&updated.executable_name),
                    );
                    self.state = S::I(updated, RS::R(running));
                    return Ok(self);
                } else {
                    log::info!(
                        "Current installation is up to date: Steam app build ID {}",
                        current.to
                    );

                    log::info!("Spawning game process...");
                    let running: RunningGameServerProcess = self.spawn(
                        &current.root_dir_absolute,
                        &current.root_dir_absolute.join(&current.executable_name),
                    );
                    self.state = S::I(current.clone(), RS::R(running));
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

            (S::I(current, RS::R(running)), T::_Stop) => {
                Game::terminate(running.pid);
                self.state = S::I(current.clone(), RS::NR);
                return Ok(self);
            }

            (S::I(current, RS::R(running)), T::_Update) => {
                let latest: SteamAppBuildId = self.query_latest_version_info()?;
                if current.to != latest {
                    Game::terminate(running.pid);
                    let updated: Updation = Game::update();
                    let running: RunningGameServerProcess = self.spawn(
                        &updated.root_dir_absolute,
                        &updated.root_dir_absolute.join(&updated.executable_name),
                    );
                    self.state = S::I(updated, RS::R(running));
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
                let running: RunningGameServerProcess = self.spawn(
                    &installed.root_dir_absolute,
                    &installed.root_dir_absolute.join(&installed.executable_name),
                );
                self.state = S::I(installed, RS::R(running));
                return Ok(self);
            }

            (S::NI, T::_Stop) => Ok(self), // Nothing to do!
        }
    }

    /* TODO: Fix querying latest version info from remote! Current
             implementation does not work!

       Observations from 2025-02-15:

       - Running "steamcmd +app_info_print 258550 +quit" returned information
         from some local cache instead of fetching from remote: buildid of
         public branch matched what was latest on 2025-01-19 when I had last
         installed the game server.

       - Removing "/home/jka/.local/share/Steam/appcache/appinfo.vdf" and then
         running "steamcmd +login anonymous +app_info_print 258550 +quit" returned
         actual latest information from remote

       So, should remove local cache first and then query with anonymous login,
       I guess!
    */
    fn query_latest_version_info(&self) -> Result<SteamAppBuildId, Error> {
        let cache_filename = std::path::PathBuf::from("appinfo.vdf");
        match crate::system::find_single_file(&cache_filename, &None) {
            Ok(n) => {
                log::debug!("Found {n}");
                todo!("remove {n}");
            }
            Err(crate::system::FindSingleFileError::FileNotFound { .. }) => {
                // nothing to wipe: no local cache exists
            }
            Err(crate::system::FindSingleFileError::ManyFilesFound {
                paths_absolute_found,
            }) => {
                return Err(Error::CannotCheckUpdates(CCU::AmbiguousLocalCache {
                    cache_filename_seeked: cache_filename,
                    cache_paths_absolute_found: paths_absolute_found,
                }))
            }
        };
        todo!();

        // let argv: Vec<std::borrow::Cow<'_, str>> =
        //     vec!["+app_info_update".into(), "1".into(), "+quit".into()];
        // self.steamcmd_exec(argv)?;

        // let argv: Vec<std::borrow::Cow<'_, str>> = vec![
        //     "+app_info_print".into(),
        //     Game::get_game_steam_app_id().to_string().into(),
        //     "+quit".into(),
        // ];
        // let stdout_utf8: String = self.steamcmd_exec(argv)?;
        // let build_id: u32 = match crate::parsing::parse_buildid_from_buffer(&stdout_utf8) {
        //     Some(n) => n,
        //     None => {
        //         return Err(Error::SteamCMDUnexpectedOutput(
        //             String::from("parse build ID"),
        //             stdout_utf8,
        //         ));
        //     }
        // };
        // return Ok(build_id);
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

        let game_executable_found: crate::system::FoundFile =
            match crate::system::find_single_file(&Game::get_game_executable_filename(), &None) {
                Ok(n) => n,
                Err(_) => todo!(),
            };

        let manifest_seekable: std::path::PathBuf = game_executable_found
            .dir_path_absolute
            .join("steamapps")
            .join(Game::get_game_manifest_filename());
        let manifest_found: crate::system::FoundFile =
            match crate::system::find_single_file(&manifest_seekable, &None) {
                Ok(n) => n,
                Err(_) => todo!(),
            };

        let build_id: u32 = match crate::parsing::parse_buildid_from_manifest(
            &manifest_found.get_absolute_path(),
        ) {
            Some(n) => n,
            None => todo!(),
        };

        let installation: Updation = Updation {
            installed_at: manifest_found.last_modified,
            from: build_id,
            to: build_id,
            root_dir_absolute: game_executable_found.dir_path_absolute,
            executable_name: game_executable_found.filename,
            _manifest_name: manifest_found.filename,
        };

        return Ok(installation);
    }

    fn update() -> Updation {
        todo!("update game server using SteamCMD");
    }

    // TODO: Define parameter driving data state: Should some or all of the
    //       data of the program be removed before spawning the process? (Namely
    //       previous game world maps, player blueprints and any other game
    //       data...)
    fn spawn(
        &self,
        work_dir: &std::path::Path,
        executable: &std::path::Path,
    ) -> RunningGameServerProcess {
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
        todo!("resolve the return values");
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
            S::I(updation, RS::R(running)) => {
                write!(f, "installed: {updation}, running as PID {}", running.pid)
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
pub struct RunningGameServerProcess {
    /// Linux process ID of the running game server process.
    pid: LinuxProcessId,

    /// RCON password configured at startup of the running game server.
    _rcon_password: String,

    /// Absolute path to the running game server instance's data directory.
    ///
    /// An example of the data directory's contents (root is relative to the
    /// game server executable):
    /// ```
    /// ./server/my_server_identity/
    /// ├── cfg
    /// │   ├── bans.cfg
    /// │   ├── serverauto.cfg
    /// │   └── users.cfg
    /// ├── player.blueprints.5.db
    /// ├── player.deaths.5.db
    /// ├── player.identities.5.db
    /// ├── player.states.263.db
    /// ├── player.tokens.db
    /// ├── proceduralmap.4500.1337.263.map
    /// └── sv.files.263.db
    /// ```
    _rds_instance_data_dir_path_absolute: std::path::PathBuf,
}

// TODO: Refactor so that "already running" is not a valid initial state (remove "Running state" altogether)
#[derive(Debug)]
/// Running state.
pub enum RS {
    /// Running.
    R(RunningGameServerProcess),
    /// Not running.
    NR,
}

fn determine_inital_state(
    executable_name: &'static std::path::Path,
    exclude_from_search: Option<std::path::PathBuf>,
) -> Result<S, crate::system::FindSingleFileError> {
    let game_executable_found: crate::system::FoundFile =
        crate::system::find_single_file(executable_name, &exclude_from_search)?;

    let manifest_seekable: std::path::PathBuf = game_executable_found
        .dir_path_absolute
        .join("steamapps")
        .join(Game::get_game_manifest_filename());
    let manifest_found: crate::system::FoundFile =
        match crate::system::find_single_file(&manifest_seekable, &exclude_from_search) {
            Ok(n) => n,
            Err(_) => return Ok(S::NI),
        };

    let build_id: u32 =
        match crate::parsing::parse_buildid_from_manifest(&manifest_found.get_absolute_path()) {
            Some(n) => n,
            None => todo!("define error case"),
        };

    let running: RS = match crate::system::check_process_running(&game_executable_found.filename) {
        Ok(None) => RS::NR,
        Ok(Some(_pid)) => {
            todo!("define error case");
        }
        Err(crate::system::IdentifySingleProcessError::LibProcfsFailure { .. }) => {
            todo!("define error case");
        }
        Err(crate::system::IdentifySingleProcessError::RunningParallel { .. }) => {
            todo!("define error case");
        }
    };

    let updation: Updation = Updation {
        installed_at: manifest_found.last_modified,
        from: build_id,
        to: build_id,
        root_dir_absolute: game_executable_found.dir_path_absolute,
        executable_name: game_executable_found.filename,
        _manifest_name: manifest_found.filename,
    };

    return Ok(S::I(updation, running));
}
