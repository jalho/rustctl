mod core {
    use tungstenite::Message;

    use crate::net::{Client, serve};
    use std::{
        collections::HashMap,
        net::SocketAddr,
        sync::{Arc, Mutex},
        thread::{Builder, JoinHandle, sleep},
        time::{Duration, SystemTime, UNIX_EPOCH},
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
            Report
        }
    }

    /// What is going to be attempted.
    pub struct Plan;

    impl Plan {
        pub fn new(_command: String) -> Self {
            Self
        }

        pub fn serialize(&self) -> Message {
            todo!();
        }
    }

    /// What happened (while trying to execute a plan).
    pub struct Report;

    impl Report {
        pub fn serialize(&self) -> Message {
            todo!();
        }
    }

    pub enum Notification<'plan, 'report> {
        Plan(&'plan Plan),
        Report(&'report Report),
    }

    impl Controller {
        pub fn new() -> Self {
            Self {
                game_state: Arc::new(Mutex::new(GameState::Initializing)),
                clients: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        pub fn start_server(&self, thread_name: &str) -> JoinHandle<()> {
            let clients: Arc<Mutex<HashMap<SocketAddr, Client>>> = self.clients.clone();

            std::thread::Builder::new()
                .name(thread_name.into())
                .spawn(move || {
                    serve(clients);
                })
                .unwrap()
        }

        /// Serve current state and available options to (commanding) clients.
        pub fn sync_state(&self, thread_name: &str) -> JoinHandle<()> {
            let game_state = self.game_state.clone();
            let clients = self.clients.clone();

            Builder::new()
                .name(thread_name.into())
                .spawn(move || {
                    loop {
                        sleep(Duration::from_millis(1));

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
                                if client.send(Message::text(&serialized)).is_err() {
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
                })
                .unwrap()
        }

        pub fn relay_commands(&self, thread_name: &str) -> JoinHandle<()> {
            let clients = self.clients.clone();
            let game_state = self.game_state.clone();

            Builder::new()
                .name(thread_name.into())
                .spawn(move || {
                    'relaying: loop {
                        sleep(Duration::from_millis(1));

                        let mut plan: Option<Plan> = None;
                        {
                            let mut clients = clients.lock().unwrap();
                            'receiving: for (_addr, client) in clients.iter_mut() {
                                if let Some(n) = client.recv_command() {
                                    plan = Some(n);
                                    break 'receiving;
                                }
                            }
                        }

                        let plan: Plan = match plan {
                            Some(n) => n,
                            None => continue 'relaying,
                        };

                        {
                            let mut clients = clients.lock().unwrap();
                            for (_addr, client) in clients.iter_mut() {
                                client.notify(Notification::Plan(&plan));
                            }
                        }

                        let report: Report;
                        {
                            let mut game_state = game_state.lock().unwrap();
                            report = game_state.transition(&plan);
                        }

                        {
                            let mut clients = clients.lock().unwrap();
                            for (_addr, client) in clients.iter_mut() {
                                client.notify(Notification::Report(&report));
                            }
                        }
                    }
                })
                .unwrap()
        }
    }
}

mod net {
    use crate::core::{Notification, Plan};
    use std::{
        collections::HashMap,
        net::{SocketAddr, TcpListener, TcpStream},
        sync::{Arc, Mutex},
    };
    use tungstenite::{
        Message,
        handshake::server::{ErrorResponse, Request, Response},
        protocol::WebSocket,
    };

    pub struct Client {
        websocket: WebSocket<TcpStream>,
    }

    #[allow(clippy::result_large_err)]
    fn websocket_handshake(
        _request: &Request,
        response: Response,
    ) -> Result<Response, ErrorResponse> {
        Ok(response)
    }

    impl Client {
        pub fn new(stream: TcpStream) -> Self {
            stream.set_nonblocking(true).unwrap();
            let websocket = tungstenite::accept_hdr(stream, websocket_handshake).unwrap();
            Self { websocket }
        }

        pub fn recv_command(&mut self) -> Option<Plan> {
            match self.websocket.read() {
                Ok(Message::Text(utf8)) => Some(Plan::new(utf8.to_string())),
                _ => None,
            }
        }

        pub fn notify(&mut self, notification: Notification) {
            match notification {
                Notification::Plan(plan) => {
                    self.websocket.send(plan.serialize()).unwrap();
                }
                Notification::Report(report) => {
                    self.websocket.send(report.serialize()).unwrap();
                }
            }
        }

        pub fn send(&mut self, serialized: Message) -> Result<(), tungstenite::Error> {
            self.websocket.send(serialized)
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

    let th_syncer = controller.sync_state("sync");

    let th_relayer = controller.relay_commands("relay");

    let th_server = controller.start_server("server");

    _ = th_syncer.join();
    _ = th_relayer.join();
    _ = th_server.join();
}
