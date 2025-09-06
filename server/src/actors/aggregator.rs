pub struct Aggregator {
    ctoken: tokio_util::sync::CancellationToken,

    rx_resuse: tokio::sync::mpsc::Receiver<crate::actors::monitor::SystemResourceUsageReading>,
    rx_gss: tokio::sync::mpsc::Receiver<rustctl_common::snapshot::GameServerStateExposed>,
    rx_igs: tokio::sync::mpsc::Receiver<rustctl_common::snapshot::InGameStateExposed>,
    aggregated: std::sync::Arc<tokio::sync::Mutex<Aggregated>>,
    tx_broadcast: tokio::sync::broadcast::Sender<rustctl_common::snapshot::Snapshot>,

    rx_cmd_collect: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
    tx_cmd_relay: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,

    game_world_size: f64,
}

impl Aggregator {
    const SOCKET_PATH: &str = "/tmp/rustctl.sock"; // TODO: Get via the config's `fs` thing instead!

    pub fn new(
        ctoken: tokio_util::sync::CancellationToken,
        rx_resuse: tokio::sync::mpsc::Receiver<crate::actors::monitor::SystemResourceUsageReading>,
        rx_gss: tokio::sync::mpsc::Receiver<rustctl_common::snapshot::GameServerStateExposed>,
        rx_igs: tokio::sync::mpsc::Receiver<rustctl_common::snapshot::InGameStateExposed>,
        rx_cmd_collect: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
        tx_cmd_relay: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
        tx_broadcast: tokio::sync::broadcast::Sender<rustctl_common::snapshot::Snapshot>,
        game_world_size: f64,
    ) -> Self {
        Self {
            ctoken,

            rx_resuse,
            rx_gss,
            rx_igs,
            aggregated: Aggregated::init(),
            tx_broadcast,

            rx_cmd_collect,
            tx_cmd_relay,

            game_world_size,
        }
    }

    pub async fn work(self) -> Summary {
        let ctoken = self.ctoken.child_token();

        /*
         * Aggregate all kinds of stuff to internal collection as soon as stuff
         * arrives from the other actors, and broadcast the aggregated on a
         * regular interval.
         */
        let job_agg_resuse = Self::aggregate_system_resources_usage_readings(self.aggregated.clone(), self.rx_resuse);
        let job_agg_gss = Self::aggregate_game_server_state_machine_transitions(self.aggregated.clone(), self.rx_gss);
        let job_agg_igs = Self::aggregate_ingame_state(self.aggregated.clone(), self.rx_igs);
        let job_agg_ige = Self::aggregate_ingame_events(self.aggregated.clone());
        let job_broadcast = Self::broadcast(self.game_world_size, self.aggregated.clone(), self.tx_broadcast);

        let job_cmd_relay = Self::relay_gsc_commands(self.rx_cmd_collect, self.tx_cmd_relay.clone());

        let _done = ctoken
            .run_until_cancelled(async {
                let done: ((), (), (), (), (), ()) = tokio::join!(
                    job_agg_resuse,
                    job_agg_gss,
                    job_agg_igs,
                    job_agg_ige,
                    job_broadcast,
                    job_cmd_relay,
                );
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
            let msg: rustctl_common::command::DownstreamClientMessage = match rx_command.recv().await {
                Some(n) => n,
                None => {
                    log::debug!(
                        "Channel for receiving relayable commands from downstream clients is closed (all senders dropped) -- Stopping relaying"
                    );
                    break 'relay;
                }
            };
            if let Err(err) = tx_command.send(msg).await {
                log::debug!(
                    "Channel for relaying commands to game server controller is closed -- Stopping relaying: {err}"
                );
                break 'relay;
            }
        }
    }

    async fn broadcast(
        game_world_size: f64,
        aggregated: std::sync::Arc<tokio::sync::Mutex<Aggregated>>,
        tx_broadcast: tokio::sync::broadcast::Sender<rustctl_common::snapshot::Snapshot>,
    ) -> () {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            let memory_used_kibibytes: u64;
            let cpus_usage: Vec<super::monitor::Percentage>;
            let game_server_state: rustctl_common::snapshot::GameServerStateExposed;
            let ingame_state: rustctl_common::snapshot::InGameStateExposed;
            {
                let lock = aggregated.lock().await;

                memory_used_kibibytes = lock.kibibytes_in_use;
                cpus_usage = lock.all_cpus.clone();
                game_server_state = lock.game_server_state.clone();
                ingame_state = lock.ingame_state.clone();
            }

            let snapshot: rustctl_common::snapshot::Snapshot = rustctl_common::snapshot::Snapshot {
                game_server_state,
                ingame_state,
                memory_used_kibibytes: rustctl_common::snapshot::MemoryUsage::new(memory_used_kibibytes),
                cpus_utilization_percentage: cpus_usage
                    .iter()
                    .map(|n| {
                        let perc: &super::monitor::Percentage = n;
                        let float: f64 = perc.into();
                        let usage: rustctl_common::snapshot::CpuUsage = rustctl_common::snapshot::CpuUsage::new(float);
                        usage
                    })
                    .collect::<Vec<rustctl_common::snapshot::CpuUsage>>(),
                game_world_size,
            };
            _ = tx_broadcast.send(snapshot);
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

    async fn aggregate_ingame_state(
        aggregated: std::sync::Arc<tokio::sync::Mutex<Aggregated>>,
        mut rx_igs: tokio::sync::mpsc::Receiver<rustctl_common::snapshot::InGameStateExposed>,
    ) -> () {
        'receive: loop {
            let received: rustctl_common::snapshot::InGameStateExposed = match rx_igs.recv().await {
                Some(n) => n,
                None => break 'receive,
            };

            let mut lock = aggregated.lock().await;
            lock.ingame_state = received;
        }
    }

    async fn aggregate_ingame_events(_aggregated: std::sync::Arc<tokio::sync::Mutex<Aggregated>>) -> () {
        let _ = tokio::fs::remove_file(Self::SOCKET_PATH).await;

        let listener = match tokio::net::UnixListener::bind(Self::SOCKET_PATH) {
            Ok(n) => n,
            Err(err) => todo!("terminate gracefully: {err}"),
        };
        log::debug!("Unix domain socket bound: {}", Self::SOCKET_PATH);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    log::debug!("New Unix domain socket connection established");
                    let mut reader = tokio::io::BufReader::new(stream);
                    let mut line = String::new();
                    'receive: loop {
                        line.clear();
                        let utf8: String = match tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut line).await {
                            Ok(0) => {
                                todo!("terminate gracefully: connection closed");
                            }
                            Ok(_) => {
                                let payload: &str = line.trim();
                                payload.to_string()
                            }
                            Err(err) => {
                                todo!("terminate gracefully: error reading from socket: {}", err);
                            }
                        };

                        let event: ige::InGameEvent = match serde_json::from_str(&utf8) {
                            Ok(n) => n,
                            Err(err) => {
                                log::error!("TODO:\n{err}:\n{utf8}");
                                continue 'receive;
                            }
                        };
                        log::debug!("In-game event: {event:?}");
                    }
                }
                Err(err) => {
                    todo!("terminate gracefully: error accepting connection: {}", err);
                }
            }
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
    ingame_state: rustctl_common::snapshot::InGameStateExposed,
}

impl Aggregated {
    pub fn init() -> std::sync::Arc<tokio::sync::Mutex<Self>> {
        std::sync::Arc::new(tokio::sync::Mutex::new(Self {
            last_updated_at: None,
            kibibytes_in_use: 0,
            all_cpus: Vec::new(),
            game_server_state: rustctl_common::snapshot::GameServerStateExposed::Init,
            ingame_state: rustctl_common::snapshot::InGameStateExposed::init(),
        }))
    }
}

mod ige {
    //! In-game events's (IGE) related stuff.

    #[derive(Debug, serde::Deserialize)]
    pub enum Resource {
        #[serde(rename = "Animal Fat")]
        AnimalFat,

        #[serde(rename = "Blue Berry")]
        BlueBerry,

        #[serde(rename = "Blue Berry Seed")]
        BlueBerrySeed,

        #[serde(rename = "Bone Fragments")]
        BoneFragments,

        Cloth,

        #[serde(rename = "Diesel Fuel")]
        DieselFuel,

        #[serde(rename = "Green Berry")]
        GreenBerry,

        #[serde(rename = "Green Berry Seed")]
        GreenBerrySeed,

        #[serde(rename = "Hemp Seed")]
        HempSeed,

        Leather,

        #[serde(rename = "Metal Ore")]
        MetalOre,

        Mushroom,

        #[serde(rename = "Raw Bear Meat")]
        RawBearMeat,

        #[serde(rename = "Raw Horse Meat")]
        RawHorseMeat,

        #[serde(rename = "Raw Pork")]
        RawPork,

        #[serde(rename = "Red Berry")]
        RedBerry,

        #[serde(rename = "Red Berry Seed")]
        RedBerrySeed,

        Stones,

        #[serde(rename = "Sulfur Ore")]
        SulfurOre,

        #[serde(rename = "White Berry")]
        WhiteBerry,

        #[serde(rename = "White Berry Seed")]
        WhiteBerrySeed,

        Wood,
    }

    #[derive(Debug, serde::Deserialize)]
    #[serde(tag = "hook")]
    pub enum InGameEvent {
        OnDispenserGather {
            steam_id: u64,

            amount: f64,
            resource: Resource,
        },
        OnDispenserBonus {
            steam_id: u64,

            amount: f64,
            resource: Resource,
        },
        OnGrowableGathered {
            steam_id: u64,

            amount: f64,
            resource: Resource,
        },
        OnCollectiblePickup {
            steam_id: u64,

            items: Vec<Item>,
        },
        OnCargoShipSpawnCrate,
    }

    #[derive(Debug, serde::Deserialize)]
    struct Item {
        resource: Resource,
        amount: f64,
    }
}
