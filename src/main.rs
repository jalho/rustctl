mod core {
    use crate::net::{Client, serve};
    use std::{
        collections::HashMap,
        net::SocketAddr,
        sync::{Arc, Mutex},
        thread::JoinHandle,
        time::{SystemTime, UNIX_EPOCH},
    };

    pub struct Controller {
        game_state: Arc<Mutex<GameState>>,
        clients: Arc<Mutex<HashMap<SocketAddr, Client>>>,
    }

    /// Game server contoller as a state machine.
    #[derive(Debug)]
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
    pub struct Plan;

    /// What happened (while trying to execute a plan).
    pub struct Report;

    pub enum Notification<'plan, 'report> {
        Plan(&'plan Plan),
        Report(&'report Report),
    }

    impl Controller {
        pub fn new() -> Self {
            return Self {
                game_state: Arc::new(Mutex::new(GameState::Initializing)),
                clients: Arc::new(Mutex::new(HashMap::new())),
            };
        }

        pub fn start_server(&self) -> JoinHandle<()> {
            let clients: Arc<Mutex<HashMap<SocketAddr, Client>>> = self.clients.clone();

            return std::thread::spawn(move || {
                serve(clients);
            });
        }

        /// Serve current state and available options to (commanding) clients.
        pub fn sync_state(&self) -> JoinHandle<()> {
            let game_state = self.game_state.clone();
            let clients = self.clients.clone();

            return std::thread::spawn(move || {
                loop {
                    let serialized: String;
                    {
                        let lock_game_state = game_state.lock().unwrap();
                        let timestamp_secs = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        serialized = format!("{timestamp_secs}: {lock_game_state:?}\n");
                    }

                    let mut dead_clients: Vec<SocketAddr> = Vec::new();
                    {
                        let mut lock_clients = clients.lock().unwrap();
                        for (addr, client) in lock_clients.iter_mut() {
                            if let Err(_) = client.send(serialized.as_bytes()) {
                                dead_clients.push(addr.to_owned());
                            }
                        }
                    }

                    {
                        let mut lock_clients = clients.lock().unwrap();
                        for addr in dead_clients.iter() {
                            lock_clients.remove(addr);
                        }
                    }
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
                    'receiving: for (_addr, client) in clients.iter() {
                        if let Some(n) = client.recv_command() {
                            plan = Some(n);
                            break 'receiving;
                        }
                    }

                    let plan: Plan = match plan {
                        Some(n) => n,
                        None => continue 'relaying,
                    };

                    for (_addr, client) in clients.iter() {
                        client.notify(Notification::Plan(&plan));
                    }

                    let report: Report = game_state.transition(&plan);

                    for (_addr, client) in clients.iter() {
                        client.notify(Notification::Report(&report));
                    }
                }
            });
        }
    }
}

mod net {
    use crate::core::{Notification, Plan};
    use std::{
        collections::HashMap,
        io::{Error, Write},
        net::{SocketAddr, TcpListener, TcpStream},
        sync::{Arc, Mutex},
    };

    pub struct Client {
        stream: TcpStream,
    }

    impl Client {
        pub fn new(stream: TcpStream) -> Self {
            return Self { stream };
        }

        /// Wait for a command (to transition state).
        pub fn recv_command(&self) -> Option<Plan> {
            return None;
        }

        pub fn notify(&self, _notification: Notification) {
            match _notification {
                Notification::Plan(_plan) => todo!(),
                Notification::Report(_report) => todo!(),
            }
        }

        pub fn send(&mut self, serialized: &[u8]) -> Result<usize, Error> {
            return self.stream.write(serialized);
        }
    }

    pub fn serve(clients: Arc<Mutex<HashMap<SocketAddr, Client>>>) {
        let listener: TcpListener = TcpListener::bind("127.0.0.1:8080").unwrap();

        loop {
            let (stream, addr): (TcpStream, SocketAddr) = listener.accept().unwrap();
            let client = Client::new(stream);

            {
                let mut clients = clients.lock().unwrap();
                clients.insert(addr, client);
            }
        }
    }
}

fn main() {
    let controller = core::Controller::new();

    let th_syncer = controller.sync_state();

    let th_relayer = controller.relay_commands();

    let th_server = controller.start_server();

    _ = th_syncer.join();
    _ = th_relayer.join();
    _ = th_server.join();
}
