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
    if let Ok(metadata) = is_hetzner_vm().await {
        log::info!("Rebooting cloud instance: {metadata}");
    } else {
        log::info!("Skipping reboot: Not in cloud");
        return;
    }

    let status = tokio::process::Command::new("systemctl")
        .arg("reboot")
        .status()
        .await
        .unwrap();

    if !status.success() {
        log::error!("Failed to reboot: systemctl exited with status code: {status}");
    }
}

async fn is_hetzner_vm() -> Result<String, reqwest::Error> {
    /*
     * Docs: https://docs.hetzner.cloud/reference/cloud
     *       Accessed 2026-01-17.
     */
    let url: &'static str = "http://169.254.169.254/hetzner/v1/metadata";

    let client: reqwest::Client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(500))
        .build()
        .unwrap();

    let response: reqwest::Response = match client.get(url).send().await {
        Ok(n) => n,
        Err(err) => return Err(err),
    };

    let instance_metadata: String = response.text().await.unwrap();

    Ok(instance_metadata)
}
