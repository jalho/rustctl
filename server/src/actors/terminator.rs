pub struct Terminator {
    ctoken: tokio_util::sync::CancellationToken,
    rx_activate: tokio::sync::mpsc::Receiver<Activator>,
}

impl Terminator {
    pub fn new(
        ctoken: tokio_util::sync::CancellationToken,
        rx_activate: tokio::sync::mpsc::Receiver<Activator>,
    ) -> Self {
        Self {
            ctoken,
            rx_activate,
        }
    }

    pub async fn work(self) -> Summary {
        let sigint: tokio::signal::unix::Signal =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()) {
                Ok(n) => n,
                Err(err) => {
                    log::error!("Failed to create signal listener for SIGINT: {err}");
                    return Summary {};
                }
            };
        return Summary {};
    }
}

pub struct Summary {}

/// Which actor is activating the global termination signal.
pub enum Activator {}
