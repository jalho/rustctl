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
        log::info!("Rebooting cloud instance: {metadata:?}");
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

async fn is_hetzner_vm() -> Result<hetzner::Metadata, reqwest::Error> {
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

    let instance_metadata_utf8: String = response.text().await.unwrap();

    let instance_metadata: hetzner::Metadata =
        serde_yaml::from_str(&instance_metadata_utf8).unwrap();

    Ok(instance_metadata)
}

mod hetzner {
    #[derive(Debug, serde::Deserialize)]
    #[allow(dead_code)]
    pub struct Metadata {
        #[serde(rename = "instance-id")]
        instance_id: u64,
        hostname: String,
        region: String,
        #[serde(rename = "availability-zone")]
        availability_zone: String,
        #[serde(rename = "local-ipv4")]
        local_ipv4: String,
        #[serde(rename = "public-ipv4")]
        public_ipv4: String,
        #[serde(rename = "network-config")]
        network_config: NetworkConfig,
        #[serde(rename = "vendor_data")]
        vendor_data: String,
        #[serde(rename = "public-keys")]
        public_keys: Vec<String>,
        runcmd: Vec<String>,
        #[serde(rename = "system_info")]
        system_info: SystemInfo,
    }

    #[derive(Debug, serde::Deserialize)]
    #[allow(dead_code)]
    struct NetworkConfig {
        version: u32,
        config: Vec<serde_yaml::Value>,
    }

    #[derive(Debug, serde::Deserialize)]
    #[allow(dead_code)]
    struct SystemInfo {
        default_user: DefaultUser,
    }

    #[derive(Debug, serde::Deserialize)]
    #[allow(dead_code)]
    struct DefaultUser {
        lock_passwd: bool,
        name: String,
        shell: String,
    }
}
