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
        return Summary {};
    }
}

pub struct Summary {}

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
pub struct Percentage(f64);

impl Percentage {
    pub fn new(value: f64) -> Self {
        Self(value)
    }
}
