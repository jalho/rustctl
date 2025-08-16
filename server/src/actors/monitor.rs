pub struct Monitor {
    ctoken: tokio_util::sync::CancellationToken,
    tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
}

impl Monitor {
    pub fn new(
        ctoken: tokio_util::sync::CancellationToken,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
    ) -> Self {
        Self { ctoken, tx_activate }
    }

    pub async fn work(self) -> Summary {
        return Summary {};
    }
}

pub struct Summary {}
