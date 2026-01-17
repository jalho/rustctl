pub enum Command {
    Reboot,
}

pub async fn handle_commands_from_web_clients(mut rx: tokio::sync::mpsc::Receiver<Command>) {
    loop {
        if let Some(n) = rx.recv().await {
            match n {
                Command::Reboot => reboot_using_systemctl().await,
            }
        }
    }
}

async fn reboot_using_systemctl() {
    let status = tokio::process::Command::new("systemctl")
        .arg("reboot")
        .status()
        .await
        .unwrap();

    if !status.success() {
        log::error!("Failed to reboot: systemctl exited with status code: {status}");
    }
}
