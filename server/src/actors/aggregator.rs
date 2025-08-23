pub struct Aggregator {
    ctoken: tokio_util::sync::CancellationToken,
    tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

    rx_resuse: tokio::sync::mpsc::Receiver<crate::actors::monitor::SystemResourceUsageReading>,
    rx_gss: tokio::sync::mpsc::Receiver<rustctl_common::snapshot::GameServerStateExposed>,
    aggregated: std::sync::Arc<tokio::sync::Mutex<Aggregated>>,

    rx_cmd_collect: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
    tx_cmd_relay: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
}

impl Aggregator {
    pub fn new(
        ctoken: tokio_util::sync::CancellationToken,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
        rx_resuse: tokio::sync::mpsc::Receiver<crate::actors::monitor::SystemResourceUsageReading>,
        rx_gss: tokio::sync::mpsc::Receiver<rustctl_common::snapshot::GameServerStateExposed>,
        rx_cmd_collect: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
        tx_cmd_relay: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
    ) -> Self {
        Self {
            ctoken,
            tx_activate,

            rx_resuse,
            rx_gss,
            aggregated: Aggregated::init(),

            rx_cmd_collect,
            tx_cmd_relay,
        }
    }

    pub async fn work(self) -> Summary {
        let ctoken = self.ctoken.child_token();

        /*
         * Aggregate all kinds of stuff to internal collection as soon as stuff
         * arrives from the other actors, and broadcast the aggregated on a
         * regular interval.
         *
         * TODO: Add jobs here:
         * - Aggregate in-game world state updates from RCON actor sent state snapshots
         * - Aggregate game server state from game server controller actor sent state transition notifications
         */
        let job_agg_resuse = Self::aggregate_system_resources_usage_readings(self.aggregated.clone(), self.rx_resuse);
        let job_agg_gss = Self::aggregate_game_server_state_machine_transitions(self.aggregated.clone(), self.rx_gss);
        let job_broadcast = Self::broadcast(self.aggregated.clone());

        let job_cmd_relay = Self::relay_gsc_commands(self.rx_cmd_collect, self.tx_cmd_relay.clone());

        let _done = ctoken
            .run_until_cancelled(async {
                let done: ((), (), (), ()) = tokio::join!(job_agg_resuse, job_agg_gss, job_broadcast, job_cmd_relay);
                done
            })
            .await;
        Summary {}
    }

    /// Relay commands from downstream WebSocket clients to Game Server Controller (GSC).
    async fn relay_gsc_commands(
        mut rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
        tx_command: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
    ) -> () {
        'relay: loop {
            let foo: rustctl_common::command::DownstreamClientMessage = match rx_command.recv().await {
                Some(n) => n,
                None => {
                    log::debug!(
                        "Channel for receiving relayable commands from downstream clients is closed (all senders dropped) -- Stopping relaying"
                    );
                    break 'relay;
                }
            };
            if let Err(err) = tx_command.send(foo).await {
                log::debug!(
                    "Channel for relaying commands to game server controller is closed -- Stopping relaying: {err}"
                );
                break 'relay;
            }
        }
    }

    async fn broadcast(aggregated: std::sync::Arc<tokio::sync::Mutex<Aggregated>>) -> () {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        'broadcast: loop {
            interval.tick().await;

            let last_updated_at: Option<std::time::SystemTime>;
            let memory_used_kibibytes: u64;
            let cpus_usage: Vec<super::monitor::Percentage>;
            let game_server_state: rustctl_common::snapshot::GameServerStateExposed;
            {
                let lock = aggregated.lock().await;
                last_updated_at = lock.last_updated_at;
                memory_used_kibibytes = lock.kibibytes_in_use;
                cpus_usage = lock.all_cpus.clone();
                game_server_state = lock.game_server_state.clone();
            }

            // TODO: Broadcast the aggregated state for the connected downstream WebSocket clients!
            dbg!(last_updated_at, memory_used_kibibytes, cpus_usage, game_server_state);
        }
    }

    async fn aggregate_system_resources_usage_readings(
        aggregated: std::sync::Arc<tokio::sync::Mutex<Aggregated>>,
        mut rx_resuse: tokio::sync::mpsc::Receiver<crate::actors::monitor::SystemResourceUsageReading>,
    ) -> () {
        'receive: loop {
            let received = match rx_resuse.recv().await {
                Some(reading) => reading,
                None => break 'receive,
            };

            let mut lock = aggregated.lock().await;
            match received {
                super::monitor::SystemResourceUsageReading::CpuUsage {
                    read_completed_by,
                    all_cpus,
                } => {
                    lock.last_updated_at = Some(read_completed_by);
                    lock.all_cpus = all_cpus;
                }
                super::monitor::SystemResourceUsageReading::MemoryUsage {
                    read_completed_by,
                    kibibytes_in_use,
                } => {
                    lock.last_updated_at = Some(read_completed_by);
                    lock.kibibytes_in_use = kibibytes_in_use;
                }
            }
        }
    }

    async fn aggregate_game_server_state_machine_transitions(
        aggregated: std::sync::Arc<tokio::sync::Mutex<Aggregated>>,
        mut rx_gss: tokio::sync::mpsc::Receiver<rustctl_common::snapshot::GameServerStateExposed>,
    ) -> () {
        'receive: loop {
            let received = match rx_gss.recv().await {
                Some(reading) => reading,
                None => break 'receive,
            };

            let mut lock = aggregated.lock().await;
            lock.game_server_state = received;
        }
    }
}

pub struct Summary {}

#[derive(Debug)]
pub struct Aggregated {
    last_updated_at: Option<std::time::SystemTime>,
    kibibytes_in_use: u64,
    all_cpus: Vec<crate::actors::monitor::Percentage>,
    game_server_state: rustctl_common::snapshot::GameServerStateExposed,
}

impl Aggregated {
    pub fn init() -> std::sync::Arc<tokio::sync::Mutex<Self>> {
        std::sync::Arc::new(tokio::sync::Mutex::new(Self {
            last_updated_at: None,
            kibibytes_in_use: 0,
            all_cpus: Vec::new(),
            game_server_state: rustctl_common::snapshot::GameServerStateExposed::Init,
        }))
    }
}
