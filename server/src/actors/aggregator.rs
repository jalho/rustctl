pub struct Aggregator {
    ctoken: tokio_util::sync::CancellationToken,
    tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
    rx_resuse: tokio::sync::mpsc::Receiver<crate::actors::monitor::SystemResourceUsageReading>,
    aggregated: Aggregated,
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
        let job = self.aggregate();
        let _done = ctoken.run_until_cancelled(job).await;
        return Summary {};
    }

    async fn aggregate(mut self) -> () {
        'receive: loop {
            let received = match self.rx_resuse.recv().await {
                Some(reading) => reading,
                None => break 'receive,
            };
            match received {
                super::monitor::SystemResourceUsageReading::CpuUsage {
                    read_completed_by,
                    all_cpus,
                } => {
                    self.aggregated.last_read = Some(read_completed_by);
                    self.aggregated.all_cpus = all_cpus;
                },
                super::monitor::SystemResourceUsageReading::MemoryUsage {
                    read_completed_by,
                    kibibytes_in_use,
                } => {
                    self.aggregated.last_read = Some(read_completed_by);
                    self.aggregated.kibibytes_in_use = kibibytes_in_use;
                },
            }
        }
    }
}

pub struct Summary {}

pub struct Aggregated {
    last_read: Option<std::time::SystemTime>,
    kibibytes_in_use: u64,
    all_cpus: Vec<crate::actors::monitor::Percentage>,
}

impl Aggregated {
    pub fn init() -> Self {
        return Self {
            last_read: None,
            kibibytes_in_use: 0,
            all_cpus: Vec::new(),
        };
    }
}
