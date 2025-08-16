pub struct Aggregator {
    ctoken: tokio_util::sync::CancellationToken,
    tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
    rx_resuse: tokio::sync::mpsc::Receiver<crate::actors::monitor::SystemResourceUsageReading>,
    aggregated: std::sync::Arc<tokio::sync::Mutex<Aggregated>>,
}

impl Aggregator {
    pub fn new(
        ctoken: tokio_util::sync::CancellationToken,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
        rx_resuse: tokio::sync::mpsc::Receiver<crate::actors::monitor::SystemResourceUsageReading>,
    ) -> Self {
        Self {
            tx_activate,
            ctoken,
            rx_resuse,
            aggregated: Aggregated::init(),
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
        let job_broadcast = Self::broadcast(self.aggregated.clone());

        let _done = ctoken
            .run_until_cancelled(async { tokio::join!(job_agg_resuse, job_broadcast) })
            .await;
        return Summary {};
    }

    async fn broadcast(aggregated: std::sync::Arc<tokio::sync::Mutex<Aggregated>>) -> () {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        'broadcast: loop {
            interval.tick().await;
            let lock = aggregated.lock().await;
            dbg!(&lock.last_read);
            dbg!(&lock.kibibytes_in_use);
            dbg!(&lock.all_cpus);
            // TODO: Broadcast the aggregated state for the connected downstream WebSocket clients!
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
                    lock.last_read = Some(read_completed_by);
                    lock.all_cpus = all_cpus;
                }
                super::monitor::SystemResourceUsageReading::MemoryUsage {
                    read_completed_by,
                    kibibytes_in_use,
                } => {
                    lock.last_read = Some(read_completed_by);
                    lock.kibibytes_in_use = kibibytes_in_use;
                }
            }
        }
    }
}

pub struct Summary {}

#[derive(Debug)]
pub struct Aggregated {
    last_read: Option<std::time::SystemTime>,
    kibibytes_in_use: u64,
    all_cpus: Vec<crate::actors::monitor::Percentage>,
}

impl Aggregated {
    pub fn init() -> std::sync::Arc<tokio::sync::Mutex<Self>> {
        return std::sync::Arc::new(tokio::sync::Mutex::new(Self {
            last_read: None,
            kibibytes_in_use: 0,
            all_cpus: Vec::new(),
        }));
    }
}
