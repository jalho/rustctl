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
        let ctoken = self.ctoken.child_token();
        let job = self.aggregate();
        let _done = ctoken.run_until_cancelled(job).await;
        return Summary {};
    }

    async fn aggregate(mut self) -> () {
        'receive: loop {
            match self.rx_resuse.recv().await {
                Some(reading) => {
                    dbg!(reading);
                },
                None => break 'receive,
            }
        }
    }
}

pub struct Summary {}
