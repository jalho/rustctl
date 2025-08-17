pub struct Monitor {
    ctoken: tokio_util::sync::CancellationToken,
    tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
    tx_resuse: tokio::sync::mpsc::Sender<SystemResourceUsageReading>,
    previous_stats: Vec<CpuStats>,
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
            previous_stats: Vec::new(),
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

            /*
             * TODO: Define an error type for memory usage read operation
             *       and request termination of the whole program upon seeing
             *       that! (Refer to the CPU stats reading corresponding
             *       implementations...)
             */
            let kibibytes_in_use: u64 = match read_memory_usage_kibibytes().await {
                Some(n) => n,
                None => break 'read_usage,
            };

            // value for each CPU
            let cpu_stats: Vec<CpuStats> = match CpuStats::read_all_cpus().await {
                Ok(n) => n,
                Err(err) => {
                    /*
                     * TODO: Request termination of the whole program upon error!
                     */
                    log::error!("Failed to read CPU stats: {err}");
                    break 'read_usage;
                }
            };

            /*
             * TODO: Calculate "CPU usage" based on comparison with previous reading!
             */
            dbg!(cpu_stats);

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

#[derive(Debug)]
enum CpuStatsError {
    InvalidLineFormat {
        invalid_line: String,
    },
    InvalidValueNotNum {
        source: std::num::ParseIntError,
        invalid_line: String,
        value_idx_header_excluded: usize,
    },
    CannotRead {
        source: std::io::Error,
        attempted_path: String,
    },
}

impl std::error::Error for CpuStatsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CpuStatsError::InvalidLineFormat { invalid_line: _ } => None,
            CpuStatsError::InvalidValueNotNum { source, .. } => Some(source),
            CpuStatsError::CannotRead { source, .. } => Some(source),
        }
    }
}

impl std::fmt::Display for CpuStatsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CpuStatsError::InvalidLineFormat { invalid_line } => write!(f, r#"invalid line format: "{invalid_line}""#),
            CpuStatsError::InvalidValueNotNum {
                source: _,
                invalid_line,
                value_idx_header_excluded: value_idx,
            } => write!(
                f,
                r#"invalid line value at idx {value_idx}: not number: "{invalid_line}""#
            ),
            CpuStatsError::CannotRead {
                source: _,
                attempted_path,
            } => write!(f, r#"failed to read: "{attempted_path}""#),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CpuStats {
    system: u64,
    user: u64,
    nice: u64,

    irq: u64,
    softirq: u64,

    steal: u64,
    guest: u64,
    guest_nice: u64,

    idle: u64,
    iowait: u64,
}

impl CpuStats {
    async fn read_all_cpus() -> Result<Vec<Self>, CpuStatsError> {
        const PATH: &str = "/proc/stat";
        let stat_content = match tokio::fs::read_to_string(PATH).await {
            Ok(n) => n,
            Err(source) => {
                return Err(CpuStatsError::CannotRead {
                    source,
                    attempted_path: PATH.to_owned(),
                });
            }
        };
        let mut cpu_stats: Vec<Self> = Vec::new();

        for line in stat_content.lines() {
            // look for lines like "cpu0", "cpu1", etc. (not the aggregate "cpu " line)
            if line.starts_with("cpu") && line.chars().nth(3).is_some_and(|c| c.is_ascii_digit()) {
                let stats: CpuStats = line.parse()?;
                cpu_stats.push(stats);
            }
        }

        Ok(cpu_stats)
    }
}

impl std::str::FromStr for CpuStats {
    type Err = CpuStatsError;

    /// Assuming format:
    /// ```
    /// cpu0 7856 2 1650 443198 226 0 23 0 0 0
    /// ```
    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let mut parts: Vec<&str> = line.split_whitespace().collect();

        parts.remove(0);
        const PARTS_AFTER_HEADER: usize = 10;

        if parts.len() != PARTS_AFTER_HEADER {
            return Err(CpuStatsError::InvalidLineFormat {
                invalid_line: line.to_owned(),
            });
        }

        let mut values: [u64; PARTS_AFTER_HEADER] = [0; PARTS_AFTER_HEADER];

        for (idx, part) in parts.iter().enumerate() {
            let part: &str = *part;
            let parsed: u64 = match part.parse() {
                Ok(n) => n,
                Err(source) => {
                    return Err(CpuStatsError::InvalidValueNotNum {
                        source,
                        invalid_line: line.to_owned(),
                        value_idx_header_excluded: idx,
                    });
                }
            };
            values[idx] = parsed;
        }

        Ok(Self {
            user: values[0],
            nice: values[1],
            system: values[2],
            idle: values[3],
            iowait: values[4],
            irq: values[5],
            softirq: values[6],
            steal: values[7],
            guest: values[8],
            guest_nice: values[9],
        })
    }
}
