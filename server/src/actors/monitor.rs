pub struct Monitor {
    ctoken: tokio_util::sync::CancellationToken,
    tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
    tx_resuse: tokio::sync::mpsc::Sender<SystemResourceUsageReading>,
}

impl Monitor {
    pub fn new(
        ctoken: tokio_util::sync::CancellationToken,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
        tx_resuse: tokio::sync::mpsc::Sender<SystemResourceUsageReading>,
    ) -> Self {
        Self {
            ctoken,
            tx_activate,
            tx_resuse,
        }
    }

    pub async fn work(self) -> Summary {
        let ctoken = self.ctoken.child_token();
        let job = self.read_resources_usage();
        ctoken.run_until_cancelled(job).await;
        Summary {}
    }

    async fn read_resources_usage(self) -> () {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        'read_usage: loop {
            interval.tick().await;
            let kibibytes_in_use: u64 = match read_memory_usage_kibibytes().await {
                Some(n) => n,
                None => break 'read_usage,
            };
            let read_completed_by: std::time::SystemTime = std::time::SystemTime::now();
            if let Err(err) = self
                .tx_resuse
                .send(SystemResourceUsageReading::MemoryUsage {
                    read_completed_by,
                    kibibytes_in_use,
                })
                .await
            {
                log::error!("Channel for sending system resources usage reading was closed unexpectedly: {err}");
                if let Err(err) = self
                    .tx_activate
                    .send(crate::actors::terminator::Activator::SystemResourcesUsageMonitor)
                    .await
                {
                    log::error!("Failed to initiate graceful shutdown: {err}");
                }
                break 'read_usage;
            };
        }
    }
}

pub struct Summary {}

#[derive(Debug)]
pub enum SystemResourceUsageReading {
    CpuUsage {
        read_completed_by: std::time::SystemTime,

        /// All the "CPUs" listed in `/proc/stat`.
        all_cpus: Vec<Percentage>,
    },

    MemoryUsage {
        read_completed_by: std::time::SystemTime,

        /// Memory used, in kibibytes.
        kibibytes_in_use: u64,
    },
}

/// In range `[0.0, 100.0]`.
#[derive(Debug)]
pub struct Percentage(f64);

impl Percentage {
    pub fn new(value: f64) -> Self {
        Self(value)
    }
}

async fn read_memory_usage_kibibytes() -> Option<u64> {
    let meminfo_content: String = match tokio::fs::read_to_string("/proc/meminfo").await {
        Ok(n) => n,
        Err(err) => {
            log::error!("Failed to read memory usage: {err}");
            return None;
        }
    };

    let mut mem_total: u64 = 0;
    let mut mem_available: u64 = 0;

    for line in meminfo_content.lines() {
        if line.starts_with("MemTotal:") {
            mem_total = parse_meminfo_line(line)?;
        } else if line.starts_with("MemAvailable:") {
            mem_available = parse_meminfo_line(line)?;
        }
        if mem_total > 0 && mem_available > 0 {
            break;
        }
    }

    Some(mem_total - mem_available)
}

/// Parse a line from `/proc/meminfo`.
fn parse_meminfo_line(line: &str) -> Option<u64> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 2 {
        log::error!("Unsupported format for /proc/meminfo line: {line}");
        return None;
    }

    let value_str = parts[1];

    match value_str.parse::<u64>() {
        Ok(value) => Some(value),
        Err(err) => {
            log::error!("Unsupported format for /proc/meminfo line: {line}: {err}");
            None
        }
    }
}
