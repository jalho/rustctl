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
            let state: S = Game::determine_inital_state();
            let game: Game = Self { state };
            let started: Game = game.transition(T::Start);
            return started;
        }

        fn transition(mut self, transition: T) -> Self {
            match (&self.state, transition) {
                (S::I(_, RS::NR), T::Install | T::Stop) => self, // Nothing to do!

                (S::I(current, RS::NR), T::Start) => {
                    let latest: SteamAppBuildId = Game::get_latest_version();
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
                    let latest: SteamAppBuildId = Game::get_latest_version();
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
                    let latest: SteamAppBuildId = Game::get_latest_version();
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

                (S::NI, T::Install | T::Update) => todo!("install"),
                (S::NI, T::Start) => todo!("install && start"),
                (S::NI, T::Stop) => self, // Nothing to do!
            }
        }

        fn determine_inital_state() -> S {
            todo!("determine initial state");
        }

        fn get_latest_version() -> SteamAppBuildId {
            todo!("");
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
