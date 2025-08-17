pub struct Monitor {
    ctoken: tokio_util::sync::CancellationToken,
    tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
    tx_resuse: tokio::sync::mpsc::Sender<SystemResourceUsageReading>,
    previous_stats: Option<AllCpusStats>,
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
            previous_stats: None,
        }
    }

    pub async fn work(self) -> Summary {
        let ctoken = self.ctoken.child_token();
        let job = self.monitor_system_resources_usage();
        ctoken.run_until_cancelled(job).await;
        Summary {}
    }

    /// Read system resources's usage (memory, CPU...) and send the readings.
    async fn monitor_system_resources_usage(mut self) -> () {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        'read_usage: loop {
            interval.tick().await;

            /*
             * Memory usage: Read from OS and send (to aggregator).
             */
            {
                let kibibytes_in_use: u64 = match read_memory_usage_kibibytes().await {
                    Ok(n) => n,
                    Err(err) => {
                        log::error!("Failed to read memory usage: {err}");
                        self.request_termination().await;
                        break 'read_usage;
                    }
                };
                let read_completed_by: std::time::SystemTime = std::time::SystemTime::now();
                let memory_reading = SystemResourceUsageReading::MemoryUsage {
                    read_completed_by,
                    kibibytes_in_use,
                };
                if let Err(err) = self.send_reading(memory_reading).await {
                    log::error!("Failed to send reading: {err}");
                    self.request_termination().await;
                    break 'read_usage;
                }
            }

            /*
             * CPU usage: Read from OS and send (to aggregator).
             */
            {
                let current_stats: AllCpusStats = match AllCpusStats::read_time_spent().await {
                    Ok(n) => n,
                    Err(err) => {
                        log::error!("Failed to read CPU stats: {err}");
                        self.request_termination().await;
                        break 'read_usage;
                    }
                };
                let read_completed_by: std::time::SystemTime = std::time::SystemTime::now();
                // calculate usage for each CPU
                let cpu_usage: Option<Vec<Percentage>> = match &self.previous_stats {
                    Some(previous) => Some(current_stats.calculate_usage_per_cpu_since(&previous)),
                    None => None,
                };
                self.previous_stats = Some(current_stats);
                if let Some(all_cpus) = cpu_usage {
                    let cpu_reading = SystemResourceUsageReading::CpuUsage {
                        read_completed_by,
                        all_cpus,
                    };
                    if let Err(err) = self.send_reading(cpu_reading).await {
                        log::error!("Failed to send reading: {err}");
                        self.request_termination().await;
                        break 'read_usage;
                    }
                }
            }
        }
    }

    async fn send_reading(
        &mut self,
        reading: SystemResourceUsageReading,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<SystemResourceUsageReading>> {
        self.tx_resuse.send(reading).await
    }

    async fn request_termination(&mut self) -> () {
        let result = self
            .tx_activate
            .send(crate::actors::terminator::Activator::SystemResourcesUsageMonitor)
            .await;
        if let Err(err) = result {
            log::error!("Failed to initiate graceful shutdown: {err}");
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
#[derive(Debug, Clone)]
pub struct Percentage(f64);

impl Percentage {
    pub fn new(value: f64) -> Self {
        /*
         * TODO: Define an error case instead of using an assert?
         */
        assert!(value >= 0.0 && value <= 100.0);
        Self(value)
    }
}

async fn read_memory_usage_kibibytes() -> Result<u64, ErrorReadingUsage> {
    const PATH: &str = "/proc/meminfo";
    let meminfo_content: String = match tokio::fs::read_to_string(PATH).await {
        Ok(n) => n,
        Err(source) => {
            return Err(ErrorReadingUsage::CannotRead {
                source,
                attempted_path: PATH.to_owned(),
            });
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

    Ok(mem_total - mem_available)
}

/// Parse a line from `/proc/meminfo`.
fn parse_meminfo_line(line: &str) -> Result<u64, ErrorReadingUsage> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 2 {
        return Err(ErrorReadingUsage::InvalidLineFormat {
            invalid_line: line.to_owned(),
        });
    }

    let value_str = parts[1];

    match value_str.parse::<u64>() {
        Ok(value) => Ok(value),
        Err(source) => {
            return Err(ErrorReadingUsage::InvalidValue {
                source,
                invalid_line: line.to_owned(),
            });
        }
    }
}

#[derive(Debug)]
enum ErrorReadingUsage {
    InvalidLineFormat {
        invalid_line: String,
    },
    InvalidValue {
        source: std::num::ParseIntError,
        invalid_line: String,
    },
    CannotRead {
        source: std::io::Error,
        attempted_path: String,
    },
}

impl std::error::Error for ErrorReadingUsage {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ErrorReadingUsage::InvalidLineFormat { invalid_line: _ } => None,
            ErrorReadingUsage::InvalidValue { source, .. } => Some(source),
            ErrorReadingUsage::CannotRead { source, .. } => Some(source),
        }
    }
}

impl std::fmt::Display for ErrorReadingUsage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorReadingUsage::InvalidLineFormat { invalid_line } => {
                write!(f, r#"invalid line format: "{invalid_line}""#)
            }
            ErrorReadingUsage::InvalidValue {
                source: _,
                invalid_line,
            } => write!(f, r#"invalid value in line: "{invalid_line}""#),
            ErrorReadingUsage::CannotRead {
                source: _,
                attempted_path,
            } => write!(f, r#"failed to read: "{attempted_path}""#),
        }
    }
}

/// Docs: `proc_stat(5)` — Linux manual page
#[derive(Debug, Clone, Copy)]
struct CpuStats {
    /// Time spent in system mode.
    system: u64,
    /// Time spent in user mode.
    user: u64,
    /// Time spent in user mode with low priority (nice).
    nice: u64,

    /// Time servicing interrupts.
    irq: u64,
    /// Time servicing softirqs.
    softirq: u64,

    /// Stolen time, which is the time spent in other operating systems when
    /// running in a virtualized environment.
    steal: u64,
    /// Time spent running a virtual CPU for guest operating systems under the
    /// control of the Linux kernel.
    guest: u64,
    /// Time spent running a niced guest (virtual CPU for guest operating
    /// systems under the control of the Linux kernel).
    guest_nice: u64,

    /// Time spent in the idle task.
    idle: u64,
    /// Time waiting for I/O to complete. This value is not reliable...
    iowait: u64,
}

impl CpuStats {
    fn total(&self) -> u64 {
        self.guest
            + self.guest_nice
            + self.idle
            + self.iowait
            + self.irq
            + self.nice
            + self.softirq
            + self.steal
            + self.system
            + self.user
    }

    fn active(&self) -> u64 {
        self.guest
            + self.guest_nice
            // + self.idle
            // + self.iowait
            + self.irq
            + self.nice
            + self.softirq
            + self.steal
            + self.system
            + self.user
    }
}

struct AllCpusStats(Vec<CpuStats>);

impl AllCpusStats {
    /// Reading from `/proc/stat`: The amount of time, measured in units of
    /// USER_HZ that specific CPUs spent in various states.
    async fn read_time_spent() -> Result<Self, ErrorReadingUsage> {
        const PATH: &str = "/proc/stat";
        let stat_content = match tokio::fs::read_to_string(PATH).await {
            Ok(n) => n,
            Err(source) => {
                return Err(ErrorReadingUsage::CannotRead {
                    source,
                    attempted_path: PATH.to_owned(),
                });
            }
        };
        let mut cpu_stats: Vec<CpuStats> = Vec::new();

        for line in stat_content.lines() {
            // look for lines like "cpu0", "cpu1", etc. (not the aggregate "cpu " line)
            if line.starts_with("cpu") && line.chars().nth(3).is_some_and(|c| c.is_ascii_digit()) {
                let stats: CpuStats = line.parse()?;
                cpu_stats.push(stats);
            }
        }

        Ok(Self(cpu_stats))
    }

    /// Calculate CPU usage based on two consecutive readings of time spent on
    /// all CPUs.
    fn calculate_usage_per_cpu_since(&self, earlier: &Self) -> Vec<Percentage> {
        let mut usage_per_cpu: Vec<Percentage> = Vec::with_capacity(self.0.len());
        for (idx, cpu) in self.0.iter().enumerate() {
            let earlier: CpuStats = earlier.0[idx];
            let total_diff: u64 = cpu.total() - earlier.total();
            let active_diff: u64 = cpu.active() - earlier.active();

            /*
             * TODO: Define error case for "calculated usage percentage not in
             *       range [0.0, 100.0]"? I.e., make the program log an error
             *       and terminate if somehow calculating unexpected values.
             *       Currently just clamping to the expected range...
             */
            let usage: f64 = ((active_diff as f64 / total_diff as f64) * 100.0).clamp(0.0, 100.0);
            usage_per_cpu.push(Percentage::new(usage));
        }
        return usage_per_cpu;
    }
}

impl std::str::FromStr for CpuStats {
    type Err = ErrorReadingUsage;

    /// Assuming format:
    /// ```
    /// cpu0 7856 2 1650 443198 226 0 23 0 0 0
    /// ```
    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let mut parts: Vec<&str> = line.split_whitespace().collect();

        parts.remove(0);
        const PARTS_AFTER_HEADER: usize = 10;

        if parts.len() != PARTS_AFTER_HEADER {
            return Err(ErrorReadingUsage::InvalidLineFormat {
                invalid_line: line.to_owned(),
            });
        }

        let mut values: [u64; PARTS_AFTER_HEADER] = [0; PARTS_AFTER_HEADER];

        for (idx, part) in parts.iter().enumerate() {
            let part: &str = *part;
            let parsed: u64 = match part.parse() {
                Ok(n) => n,
                Err(source) => {
                    return Err(ErrorReadingUsage::InvalidValue {
                        source,
                        invalid_line: line.to_owned(),
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
