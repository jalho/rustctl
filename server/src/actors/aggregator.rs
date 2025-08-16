pub struct Aggregator {
    ctoken: tokio_util::sync::CancellationToken,
    tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
}

impl Aggregator {
    pub fn new(
        ctoken: tokio_util::sync::CancellationToken,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
    ) -> Self {
        Self { tx_activate, ctoken }
    }

    pub async fn work(self) -> Summary {
        return Summary {};
    }
}

pub struct Summary {}
