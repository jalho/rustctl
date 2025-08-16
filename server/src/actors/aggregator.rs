pub struct Aggregator {
    ctoken: tokio_util::sync::CancellationToken,
    tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
    rx_resuse: tokio::sync::mpsc::Receiver<crate::actors::monitor::SystemResourceUsageReading>,
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
        }
    }

    pub async fn work(self) -> Summary {
        return Summary {};
    }
}

pub struct Summary {}
