pub struct Terminator {
    ctoken: tokio_util::sync::CancellationToken,
    rx_activate: tokio::sync::mpsc::Receiver<Activator>,
}

impl Terminator {
    pub fn new(
        ctoken: tokio_util::sync::CancellationToken,
        rx_activate: tokio::sync::mpsc::Receiver<Activator>,
    ) -> Self {
        Self { ctoken, rx_activate }
    }

    pub async fn work(mut self) -> Summary {
        let mut sigint: tokio::signal::unix::Signal =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()) {
                Ok(n) => n,
                Err(err) => {
                    log::error!("Failed to create signal listener for SIGINT: {err}");
                    return Summary {};
                }
            };

        let mut sigterm: tokio::signal::unix::Signal =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(n) => n,
                Err(err) => {
                    log::error!("Failed to create signal listener for SIGTERM: {err}");
                    return Summary {};
                }
            };

        let job = async {
            tokio::select! {
                _ = sigint.recv() => {
                    log::debug!("Termination token activation by SIGINT");
                    self.ctoken.cancel();
                },
                _ = sigterm.recv() => {
                    log::debug!("Termination token activation by SIGTERM");
                    self.ctoken.cancel();
                },
                received = self.rx_activate.recv() => {
                    match received {
                        Some(activator) => {
                            log::debug!("Termination token activation by {activator:?}");
                            self.ctoken.cancel();
                        },
                        None => {
                            /*
                             * The `None` variant happens when all the channel's
                             * senders, i.e. the other actors of the program,
                             * have been dropped. I guess I'm gonna call that
                             * "actor exhaust" :D
                             */
                            log::debug!("Termination token activation by actor exhaust");
                            self.ctoken.cancel();
                        },
                    }
                }
            }
        };
        let _done: () = job.await;

        Summary {}
    }
}

pub struct Summary {}

/// Which actor is activating the global termination signal.
#[derive(Debug)]
pub enum Activator {
    SystemResourcesUsageMonitor,
    GameServerStateMachine,
}
