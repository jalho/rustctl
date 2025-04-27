mod core {
    use std::{
        sync::{Arc, Mutex},
        thread::JoinHandle,
    };

    pub struct Controller {
        game_state: Arc<Mutex<GameState>>,
        clients: Arc<Mutex<Vec<Client>>>,
    }

    /// Game server contoller as a state machine.
    enum GameState {
        /// Determining initial state is in progress.
        Initializing,
        /*
         * TODO: Define states & implement transitions using libraries: Some ideas for states:
         * - NotRunning: There is no game server process.
         * - Updating: Game server is being updated.
         * - Starting: Game server process has been spawned. Game is starting but not yet playable.
         * - RunningHealthy: Game server is up. Game is playable.
         */
    }

    impl GameState {
        /// Transition from one state to another.
        fn transition(&mut self, _plan: &Plan) -> Report {
            return Report;
        }
    }

    /// What is going to be attempted.
    struct Plan;

    /// What happened (while trying to execute a plan).
    struct Report;

    enum Notification<'plan, 'report> {
        Plan(&'plan Plan),
        Report(&'report Report),
    }

    struct Client;

    impl Client {
        /// Wait for a command (to transition state).
        fn recv_command(&self) -> Option<Plan> {
            return Some(Plan);
        }

        fn notify(&self, _notification: Notification) {
            match _notification {
                Notification::Plan(_plan) => todo!(),
                Notification::Report(_report) => todo!(),
            }
        }
    }

    impl Controller {
        pub fn new() -> Self {
            return Self {
                game_state: Arc::new(Mutex::new(GameState::Initializing)),
                clients: Arc::new(Mutex::new(Vec::new())),
            };
        }

        pub fn accept_clients(&self) -> JoinHandle<()> {
            let clients = self.clients.clone();

            return std::thread::spawn(move || {
                loop {
                    let _clients = clients.lock().unwrap();
                }
            });
        }

        /// Serve current state and available options to (commanding) clients.
        pub fn sync_state(&self) -> JoinHandle<()> {
            let game_state = self.game_state.clone();

            return std::thread::spawn(move || {
                loop {
                    let _game_state = game_state.lock().unwrap();
                }
            });
        }

        pub fn relay_commands(&self) -> JoinHandle<()> {
            let clients = self.clients.clone();
            let game_state = self.game_state.clone();

            return std::thread::spawn(move || {
                'relaying: loop {
                    let clients = clients.lock().unwrap();
                    let mut game_state = game_state.lock().unwrap();

                    let mut plan: Option<Plan> = None;
                    'receiving: for client in clients.iter() {
                        if let Some(n) = client.recv_command() {
                            plan = Some(n);
                            break 'receiving;
                        }
                    }

                    let plan: Plan = match plan {
                        Some(n) => n,
                        None => continue 'relaying,
                    };

                    for client in clients.iter() {
                        client.notify(Notification::Plan(&plan));
                    }

                    let report: Report = game_state.transition(&plan);

                    for client in clients.iter() {
                        client.notify(Notification::Report(&report));
                    }
                }
            });
        }
    }
}

fn main() {
    let controller = core::Controller::new();

    let th_syncer = controller.sync_state();

    let th_relayer = controller.relay_commands();

    let th_server = controller.accept_clients();

    _ = th_syncer.join();
    _ = th_relayer.join();
    _ = th_server.join();
}
