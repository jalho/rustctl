pub enum Command {
    Reboot,
}

pub async fn handle_commands_from_web_clients(mut rx: tokio::sync::mpsc::Receiver<Command>) {
    loop {
        if let Some(n) = rx.recv().await {
            match n {
                Command::Reboot => reboot().await,
            }
        }
    }
}

async fn reboot() {
    log::debug!("TODO: Reboot");
}
