static EXIT_OK: u8 = 0;
static EXIT_ERR_LOGGER: u8 = 42;
static EXIT_ERR_OTHER: u8 = 43;

fn main() -> std::process::ExitCode {
    let cli: crate::parsers::Cli = <crate::parsers::Cli as clap::Parser>::parse();

    let _handle: log4rs::Handle = match crate::logger::init_logger() {
        Ok(n) => n,
        Err(err) => {
            eprintln!("{}", crate::misc::aggregate_error_tree(&err, 2));
            return std::process::ExitCode::from(EXIT_ERR_LOGGER);
        }
    };

    match cli.subcommand {
        crate::parsers::Subcommand::GameStart { exclude } => {
            let game: crate::game::Game = match crate::game::Game::start(exclude) {
                Ok(n) => n,
                Err(err) => {
                    log::error!(
                        "Cannot start game: {}",
                        crate::misc::aggregate_error_tree(&err, 2)
                    );
                    return std::process::ExitCode::from(EXIT_ERR_OTHER);
                }
            };
            log::info!("Game started: {game}");
        }
    }

    return std::process::ExitCode::from(EXIT_OK);
}

mod misc {
    pub fn aggregate_error_tree<Error: std::error::Error + 'static>(
        error: &Error,
        indent_step: usize,
    ) -> String {
        let mut next: Option<&(dyn std::error::Error)> = Some(error);
        let mut gen: usize = 0;
        let mut aggregated: String = String::new();
        while let Some(node) = next {
            let prefix_len: usize = gen * indent_step;
            let mut indent: String = String::with_capacity(prefix_len);
            for _ in 0..prefix_len {
                indent.push(' ');
            }
            aggregated.push_str(&indent);
            aggregated.push_str(&format!("{}", node));
            aggregated.push('\n');
            next = node.source();
            gen = gen + 1;
        }
        return aggregated;
    }
}

mod game {
    pub struct Game {
        state: S,
    }

    impl Game {
        pub fn start(
            exclude_from_search: Option<std::path::PathBuf>,
        ) -> Result<Self, crate::fs::Error> {
            log::debug!("Determining initial state...");
            let state: S =
                Game::determine_inital_state("RustDedicated", 258550, exclude_from_search)?;
            log::debug!("Initial state determined: {state}");
            let game: Game = Self { state };
            let started: Game = game.transition(T::Start);
            return Ok(started);
        }

        fn transition(mut self, transition: T) -> Self {
            match (&self.state, transition) {
                (S::I(_, RS::NR), T::_Install | T::_Stop) => self, // Nothing to do!

                (S::I(current, RS::NR), T::Start) => {
                    let latest: SteamAppBuildId = Game::query_latest_version_info();
                    if current.to != latest {
                        let updated: Updation = Game::update();
                        let pid: LinuxProcessId = Game::spawn();
                        self.state = S::I(updated, RS::R(pid));
                        return self;
                    } else {
                        let pid: LinuxProcessId = Game::spawn();
                        self.state = S::I(current.clone(), RS::R(pid));
                        return self;
                    }
                }

                (S::I(current, RS::NR), T::_Update) => {
                    let latest: SteamAppBuildId = Game::query_latest_version_info();
                    if current.to != latest {
                        let updated: Updation = Game::update();
                        self.state = S::I(updated, RS::NR);
                        return self;
                    } else {
                        return self;
                    }
                }

                (S::I(_, RS::R(_)), T::_Install | T::Start) => self, // Nothing to do!

                (S::I(current, RS::R(pid)), T::_Stop) => {
                    Game::terminate(*pid);
                    self.state = S::I(current.clone(), RS::NR);
                    return self;
                }

                (S::I(current, RS::R(pid)), T::_Update) => {
                    let latest: SteamAppBuildId = Game::query_latest_version_info();
                    if current.to != latest {
                        Game::terminate(*pid);
                        let updated: Updation = Game::update();
                        let pid: LinuxProcessId = Game::spawn();
                        self.state = S::I(updated, RS::R(pid));
                        return self;
                    } else {
                        return self;
                    }
                }

                (S::NI, T::_Install | T::_Update) => {
                    let installed: Updation = Game::install();
                    self.state = S::I(installed, RS::NR);
                    return self;
                }

                (S::NI, T::Start) => {
                    let installed: Updation = Game::install();
                    let pid: LinuxProcessId = Game::spawn();
                    self.state = S::I(installed, RS::R(pid));
                    return self;
                }

                (S::NI, T::_Stop) => self, // Nothing to do!
            }
        }

        fn determine_inital_state(
            executable_name: &'static str,
            steam_app_id: u32,
            exclude_from_search: Option<std::path::PathBuf>,
        ) -> Result<S, crate::fs::Error> {
            let installed: crate::fs::ExistingFile =
                match crate::fs::find_single_file(executable_name, exclude_from_search) {
                    Ok(Some(n)) => n,
                    Ok(None) => return Ok(S::NI),
                    Err(crate::fs::Error::FileNotFound(_)) => return Ok(S::NI),
                    Err(err) => return Err(err),
                };

            let manifest_path: std::path::PathBuf = installed
                .absolute_path_parent
                .join("steamapps")
                .join(format!("appmanifest_{steam_app_id}.acf"));
            let manifest: crate::fs::ExistingFile =
                match crate::fs::ExistingFile::check(&manifest_path) {
                    Ok(n) => n,
                    Err(_) => return Ok(S::NI),
                };

            let updation: Updation = Updation {
                _completed: manifest.last_change,
                _from: None,
                to: crate::parsers::parse_buildid_from_manifest(&manifest.absolute_path_file)
                    .expect("no build ID in manifest"),
                _root_dir_absolute: installed.absolute_path_parent,
                _executable_name: std::path::PathBuf::from(executable_name),
                _manifest_name: std::path::Path::new(
                    &manifest.file_name.to_string_lossy().into_owned(),
                )
                .to_path_buf(),
            };

            let running: RS = {
                let executable: &str = "pgrep";
                let argv: Vec<std::borrow::Cow<'static, str>> = vec![executable_name.into()];
                let output: std::process::Output = match std::process::Command::new(executable)
                    .args(argv.iter().map(std::borrow::Cow::as_ref))
                    .output()
                {
                    Ok(n) => n,
                    Err(err) => {
                        return Err(crate::fs::Error::ExecutableSpawnFailed(
                            crate::fs::ExecuteAttempt {
                                executable,
                                argv,
                                predicate_display: "spawn command".into(),
                                source: err,
                            },
                        ))
                    }
                };
                if !output.status.success() {
                    RS::NR
                } else {
                    let stdout_utf8 = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                    let pid: LinuxProcessId = match str::parse::<u32>(&stdout_utf8) {
                        Ok(n) => n,
                        Err(err) => {
                            todo!("invalid output from {executable}: {err}: {stdout_utf8}")
                        }
                    };
                    RS::R(pid)
                }
            };

            return Ok(S::I(updation, running));
        }

        fn query_latest_version_info() -> SteamAppBuildId {
            todo!("query information of latest version of game server available using SteamCMD");
        }

        fn install() -> Updation {
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

    type SteamAppBuildId = u32;

    type LinuxProcessId = u32;

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
    enum RS {
        /// Running.
        R(LinuxProcessId),
        /// Not running.
        NR,
    }
}

mod fs {
    #[derive(Debug)]
    pub enum Error {
        /// Contains name or path (relative or absolute) of the file that was
        /// not found, and possible associated underlying system IO error.
        FileNotFound((std::path::PathBuf, Option<std::io::Error>)),
        MultipleFilesFound(Vec<std::path::PathBuf>),
        ExecutableSpawnFailed(ExecuteAttempt),
    }
    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::FileNotFound((_, Some(err))) => Some(err),
                Error::FileNotFound((_, None)) => None,
                Error::MultipleFilesFound(_) => None,
                Error::ExecutableSpawnFailed(err) => Some(&err.source),
            }
        }
    }
    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::FileNotFound((file, Some(err))) => {
                    write!(f, "file not found: {}: {err}", file.to_string_lossy())
                }
                Error::FileNotFound((file, None)) => {
                    write!(f, "file not found: {}", file.to_string_lossy())
                }
                Error::MultipleFilesFound(found) => {
                    let found: Vec<String> = found
                        .iter()
                        .map(|n| n.to_string_lossy().into_owned())
                        .collect::<Vec<String>>();
                    return write!(
                        f,
                        "unexpected more than 1 files found: {}",
                        found.join(", ")
                    );
                }
                Error::ExecutableSpawnFailed(attempt) => {
                    write!(
                        f,
                        "failed to {}: command failed: {} {}",
                        attempt.predicate_display,
                        attempt.executable,
                        attempt.argv.join(" ")
                    )
                }
            }
        }
    }

    #[derive(Debug)]
    pub struct ExecuteAttempt {
        pub executable: &'static str,
        pub argv: Vec<std::borrow::Cow<'static, str>>,
        /// Describes what was being attempted, formatted for inclusion in an error message.
        pub predicate_display: std::borrow::Cow<'static, str>,
        pub source: std::io::Error,
    }

    pub struct ExistingFile {
        pub file_name: std::path::PathBuf,
        pub absolute_path_file: std::path::PathBuf,
        pub absolute_path_parent: std::path::PathBuf,
        pub last_change: chrono::DateTime<chrono::Utc>,
    }
    impl ExistingFile {
        pub fn check(path: &std::path::Path) -> Result<Self, Error> {
            let metadata: std::fs::Metadata = match path.metadata() {
                Ok(n) => n,
                Err(err) => return Err(Error::FileNotFound((path.into(), Some(err)))),
            };
            let absolute_path_file: std::path::PathBuf = match path.canonicalize() {
                Ok(n) => n,
                Err(err) => return Err(Error::FileNotFound((path.into(), Some(err)))),
            };
            let absolute_path_parent: std::path::PathBuf = match path.parent() {
                Some(n) => n.into(),
                None => unreachable!("absolute path to a file should have a parent"),
            };
            let file_name: std::path::PathBuf = match absolute_path_file.file_name() {
                Some(n) => std::path::PathBuf::from(n),
                None => unreachable!("absolute path to a file should have a file name"),
            };
            let ctime: i64 = std::os::unix::fs::MetadataExt::ctime(&metadata);
            let last_change: chrono::DateTime<chrono::Utc> =
                match chrono::DateTime::from_timestamp(ctime, 0) {
                    Some(n) => n,
                    None => {
                        unreachable!("ctime of an existing file should be a valid timestamp")
                    }
                };

            return Ok(Self {
                absolute_path_file,
                absolute_path_parent,
                file_name,
                last_change,
            });
        }
    }

    pub fn find_single_file(
        executable_name: &'static str,
        exclude_from_search: Option<std::path::PathBuf>,
    ) -> Result<Option<ExistingFile>, Error> {
        let mut matches: Vec<std::path::PathBuf> = Vec::new();

        if let None = exclude_from_search {
            log::debug!("Doing a full system wide search for a file named {executable_name}... This might take a while");
        }
        for entry in walkdir::WalkDir::new("/")
            .into_iter()
            .filter_entry(|e| {
                if let Some(ref exclude_path) = exclude_from_search {
                    if e.path().starts_with(exclude_path) {
                        return false;
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
        {
            let entry: walkdir::DirEntry = entry;

            if entry.file_name() == executable_name && entry.file_type().is_file() {
                matches.push(entry.path().to_path_buf());
            }

            if matches.len() > 1 {
                return Err(Error::MultipleFilesFound(matches));
            }
        }

        match matches.len() {
            0 => Err(Error::FileNotFound((executable_name.into(), None))),
            1 => {
                let path: std::path::PathBuf = matches
                    .into_iter()
                    .next()
                    .expect("iterator of length 1 should have a first next");
                let file: ExistingFile = ExistingFile::check(&path)?;
                return Ok(Some(file));
            }
            _ => unreachable!("iterator should have length 0 or 1 at this point"),
        }
    }
}

mod logger {
    #[derive(Debug)]
    pub enum Error {
        Cfg(log4rs::config::runtime::ConfigErrors),
        Set(log::SetLoggerError),
    }
    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Cfg(err) => Some(err),
                Error::Set(err) => Some(err),
            }
        }
    }
    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "logger initialization failed")
        }
    }
    impl From<log4rs::config::runtime::ConfigErrors> for Error {
        fn from(value: log4rs::config::runtime::ConfigErrors) -> Self {
            Self::Cfg(value)
        }
    }
    impl From<log::SetLoggerError> for Error {
        fn from(value: log::SetLoggerError) -> Self {
            Self::Set(value)
        }
    }

    fn make_logger_config() -> Result<log4rs::Config, log4rs::config::runtime::ConfigErrors> {
        let stdout: log4rs::append::console::ConsoleAppender =
            log4rs::append::console::ConsoleAppender::builder()
                .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
                    "[{d(%Y-%m-%dT%H:%M:%S)}] {h([{l}])} [{t}] - {m}{n}",
                )))
                .build();

        let logger_config: log4rs::Config = log4rs::Config::builder()
            .appender(log4rs::config::Appender::builder().build("stdout", Box::new(stdout)))
            .build(
                log4rs::config::Root::builder()
                    .appender("stdout")
                    .build(log::LevelFilter::Trace),
            )?;

        return Ok(logger_config);
    }

    /// Initialize a global logging utility.
    pub fn init_logger() -> Result<log4rs::Handle, Error> {
        let config: log4rs::Config = make_logger_config()?;
        let handle: log4rs::Handle = log4rs::init_config(config)?;
        return Ok(handle);
    }
}

mod parsers {
    pub fn parse_buildid_from_manifest(manifest_path: &std::path::Path) -> Option<u32> {
        if let Ok(content) = std::fs::read_to_string(manifest_path) {
            for line in content.lines() {
                let trimmed: &str = line.trim();
                if trimmed.starts_with("\"buildid\"") {
                    if let Some(_) = trimmed.find('\"') {
                        let parts: Vec<&str> = trimmed.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(buildid) = parts[1].trim_matches('"').parse::<u32>() {
                                return Some(buildid);
                            }
                        }
                    }
                }
            }
        }
        return None;
    }

    #[derive(clap::Parser)]
    pub struct Cli {
        #[command(subcommand)]
        pub subcommand: Subcommand,
    }

    #[derive(clap::Subcommand)]
    pub enum Subcommand {
        GameStart {
            #[arg(
            long,
            help = "Exclude a directory from the game start process's search for the game executable.",
            long_help = r#"Exclude a directory from the game start process's search for the game
executable. This is useful, for example, when developing on WSL (Windows
Subsystem for Linux), where performing a whole system wide search tends to be
particularly slow. In such cases, you may want to exclude `/mnt/c/`"#,
            value_name = "DIRECTORY",
            default_value = None
        )]
            exclude: Option<std::path::PathBuf>,
        },
    }
}
