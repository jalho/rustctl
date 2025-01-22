mod misc;

fn main() -> std::process::ExitCode {
    crate::misc::init_logger();

    let game = crate::game::Game::init();
    game.start();

    return std::process::ExitCode::SUCCESS;
}

mod game {

    pub struct Game {
        state: S,
    }

    impl Game {
        pub fn init() -> Self {
            return Self {
                state: Game::determine_inital_state(),
            };
        }

        pub fn start(&self) {
            self.transition(T::Start);
        }

        fn transition(&self, transition: T) {
            match (&self.state, transition) {
                (S::NI, T::Install | T::Update) => todo!("install"),
                (S::NI, T::Start) => todo!("install && start"),
                (S::NI, T::Stop) => {} // Nothing to do!
                (S::I(_, RS::R(_)), T::Install | T::Start) => {} // Nothing to do!
                (S::I(_, RS::R(_)), T::Stop) => todo!("stop"),
                (S::I(_, RS::R(_)), T::Update) => todo!("stop && update_cond(meta)"),
                (S::I(_, RS::NR), T::Install | T::Stop) => {} // Nothing to do!
                (S::I(_, RS::NR), T::Start) => todo!("start"),
                (S::I(_, RS::NR), T::Update) => todo!("update_cond(meta)"),
            }
        }

        fn determine_inital_state() -> S {
            todo!("determine initial state");
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
