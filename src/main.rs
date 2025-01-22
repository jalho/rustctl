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
            let state = Game::determine_inital_state();
            let game = Self { state };
            game.transition(T::Start);
            return game;
        }

        fn transition(&self, transition: T) {
            match (&self.state, transition) {
                (S::I(_, RS::NR), T::Install | T::Stop) => {} // Nothing to do!
                (S::I(_, RS::NR), T::Start) => todo!("update ? update && start : start"),
                (S::I(_, RS::NR), T::Update) => todo!("update"),

                (S::I(_, RS::R(_)), T::Install | T::Start) => {} // Nothing to do!
                (S::I(_, RS::R(_)), T::Stop) => todo!("stop"),
                (S::I(_, RS::R(_)), T::Update) => todo!("update ? stop && start : noop"),

                (S::NI, T::Install | T::Update) => todo!("install"),
                (S::NI, T::Start) => todo!("install && start"),
                (S::NI, T::Stop) => {} // Nothing to do!
            }
        }

        fn determine_inital_state() -> S {
            todo!("determine initial state");
        }

        fn install() -> Updation {
            todo!("install game server using SteamCMD");
        }

        fn update(&self) -> Option<Updation> {
            todo!("check for updates and update if necessary using SteamCMD");
        }

        fn spawn() -> LinuxProcessId {
            todo!("launch RustDedicated");
        }

        fn terminate(pid: LinuxProcessId) {
            todo!("terminate RustDedicated");
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
