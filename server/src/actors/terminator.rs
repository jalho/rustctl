pub struct Terminator {
    cancellation_token: tokio_util::sync::CancellationToken,
    rx_activate: tokio::sync::mpsc::Receiver<Activator>,
}

impl Terminator {
    pub fn new(
        cancellation_token: tokio_util::sync::CancellationToken,
        rx_activate: tokio::sync::mpsc::Receiver<Activator>,
    ) -> Self {
        Self {
            cancellation_token,
            rx_activate,
        }
    }

    pub async fn work(self) -> Summary {
        return Summary {};
    }
}

pub struct Summary {}

/// Which actor is activating the global termination signal.
pub enum Activator {}
