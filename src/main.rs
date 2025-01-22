fn main() -> std::process::ExitCode {
    crate::logger::init_logger();

    let game = crate::game::Game::start();
    log::info!("Game started: {game}");

    return std::process::ExitCode::SUCCESS;
}

mod game {
    pub struct Game {
        state: S,
    }

    impl Game {
        pub fn start() -> Self {
            let state: S =
                Game::determine_inital_state(std::path::Path::new("RustDedicated"), 258550);
            let game: Game = Self { state };
            let started: Game = game.transition(T::Start);
            return started;
        }

        fn transition(mut self, transition: T) -> Self {
            match (&self.state, transition) {
                (S::I(_, RS::NR), T::Install | T::Stop) => self, // Nothing to do!

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

                (S::I(current, RS::NR), T::Update) => {
                    let latest: SteamAppBuildId = Game::query_latest_version_info();
                    if current.to != latest {
                        let updated: Updation = Game::update();
                        self.state = S::I(updated, RS::NR);
                        return self;
                    } else {
                        return self;
                    }
                }

                (S::I(_, RS::R(_)), T::Install | T::Start) => self, // Nothing to do!

                (S::I(current, RS::R(pid)), T::Stop) => {
                    Game::terminate(*pid);
                    self.state = S::I(current.clone(), RS::NR);
                    return self;
                }

                (S::I(current, RS::R(pid)), T::Update) => {
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

                (S::NI, T::Install | T::Update) => {
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

                (S::NI, T::Stop) => self, // Nothing to do!
            }
        }

        fn determine_inital_state(
            executable_name: &'static std::path::Path,
            steam_app_id: u32,
        ) -> S {
            let installation_maybe: S = {
                let executable: &str = "find";
                let needle: String = executable_name.to_string_lossy().into_owned();
                let argv: Vec<&str> = vec!["/", "-name", &needle, "-type", "f"];
                let output: std::process::Output =
                    match std::process::Command::new(executable).args(argv).output() {
                        Ok(n) => n,
                        Err(err) => todo!("could not {executable}: {err}"),
                    };
                if !output.status.success() {
                    S::NI
                } else {
                    let stdout_utf8: String =
                        String::from_utf8_lossy(&output.stdout).trim().to_owned();
                    if stdout_utf8.lines().count() != 1 {
                        let display: &str = &executable_name.to_string_lossy();
                        todo!("multiple installations of {display}: {stdout_utf8}");
                    } else {
                        let installed: String = stdout_utf8
                            .lines()
                            .last()
                            .expect("checked above: count() == 1")
                            .to_owned();
                        let installed: std::path::PathBuf =
                            std::path::Path::new(&installed).to_path_buf();
                        let parent: std::path::PathBuf = installed
                            .parent()
                            .expect("guaranteed by the way find was called: -type f")
                            .to_path_buf();
                        let manifest: std::path::PathBuf = parent
                            .join("steamapps")
                            .join(format!("appmanifest_{steam_app_id}.acf"));
                        if !manifest.is_file() {
                            S::NI
                        } else {
                            let meta: std::fs::Metadata =
                                manifest.metadata().expect("checked to be file above");
                            let ctime: i64 = std::os::linux::fs::MetadataExt::st_ctime(&meta);
                            let install_instant: chrono::DateTime<chrono::Utc> =
                                chrono::DateTime::from_timestamp(ctime, 0)
                                    .expect("weird ctime in manifest");
                            S::I(
                                Updation {
                                    completed: install_instant,
                                    from: None,
                                    to: crate::parsers::parse_buildid_from_manifest(&manifest)
                                        .expect("no build ID in manifest"),
                                    root_dir: parent,
                                    executable_name: executable_name.to_path_buf(),
                                    manifest_name: std::path::Path::new(
                                        &manifest
                                            .file_name()
                                            .expect("constructed above")
                                            .to_string_lossy()
                                            .into_owned(),
                                    )
                                    .to_path_buf(),
                                },
                                RS::NR,
                            )
                        }
                    }
                }
            };

            match installation_maybe {
                S::NI => return installation_maybe,
                S::I(installed, _) => {
                    let running: RS = {
                        let executable: &str = "pgrep";
                        let arg: String = executable_name.to_string_lossy().into_owned();
                        let argv: Vec<&str> = vec![&arg];
                        let output: std::process::Output =
                            match std::process::Command::new(executable).args(argv).output() {
                                Ok(n) => n,
                                Err(err) => todo!("could not {executable}: {err}"),
                            };
                        if !output.status.success() {
                            RS::NR
                        } else {
                            let stdout_utf8 =
                                String::from_utf8_lossy(&output.stdout).trim().to_owned();
                            let pid: LinuxProcessId = match str::parse::<u32>(&stdout_utf8) {
                                Ok(n) => n,
                                Err(err) => {
                                    todo!("invalid output from {executable}: {err}: {stdout_utf8}")
                                }
                            };
                            RS::R(pid)
                        }
                    };
                    return S::I(installed, running);
                }
            }
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

        fn terminate(pid: LinuxProcessId) {
            todo!("terminate game server process");
        }
    }

    impl std::fmt::Display for Game {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            return write!(f, "");
        }
    }

    /// State of the machine.
    enum S {
        /// Not installed.
        NI,
        /// Installed.
        I(Updation, RS),
    }

    /// Transition of the state machine.
    pub enum T {
        Install,
        Start,
        Stop,
        Update,
    }

    type SteamAppBuildId = u32;

    type LinuxProcessId = u32;

    #[derive(Clone)]
    struct Updation {
        completed: chrono::DateTime<chrono::Utc>,
        from: Option<SteamAppBuildId>,
        to: SteamAppBuildId,
        root_dir: std::path::PathBuf,
        executable_name: std::path::PathBuf,
        manifest_name: std::path::PathBuf,
    }

    /// Running state.
    enum RS {
        /// Running.
        R(LinuxProcessId),
        /// Not running.
        NR,
    }
}

mod logger {
    fn make_logger_config() -> log4rs::Config {
        let stdout: log4rs::append::console::ConsoleAppender =
            log4rs::append::console::ConsoleAppender::builder()
                .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
                    "[{d(%Y-%m-%dT%H:%M:%S)}] {h([{l}])} [{t}] - {m}{n}",
                )))
                .build();

        let logger_config: log4rs::Config = match log4rs::Config::builder()
            .appender(log4rs::config::Appender::builder().build("stdout", Box::new(stdout)))
            .build(
                log4rs::config::Root::builder()
                    .appender("stdout")
                    .build(log::LevelFilter::Trace),
            ) {
            Ok(n) => n,
            Err(_) => {
                unreachable!("logger configuration does not depend on any input so it should be either always valid or never valid");
            }
        };

        return logger_config;
    }

    /// Initialize a global logging utility.
    pub fn init_logger() -> log4rs::Handle {
        let config: log4rs::Config = make_logger_config();
        let logger: log4rs::Handle = match log4rs::init_config(config) {
            Ok(n) => n,
            Err(_) => {
                unreachable!(
                    "logger initialization should always succeed because we only do it once"
                );
            }
        };
        return logger;
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
}
