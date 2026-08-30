pub const UNIX_DOMAIN_SOCKET: &'static str = "/tmp/rustctl.sock";

pub struct GameServerConfig {
    pub install_dir: std::path::PathBuf,
}

impl Default for GameServerConfig {
    fn default() -> Self {
        Self {
            install_dir: std::path::PathBuf::from("/srv/rustctl/game/RustDedicated"),
        }
    }
}

/// This function does these steps.
///
/// - The function checks that `steamcmd` is present. If `steamcmd` is not
///   present, the function stops with an error code.
/// - The function installs or updates `RustDedicated` with `steamcmd`.
/// - The function downloads and unpacks the Carbon modding framework from
///   GitHub.
/// - The function writes the custom Carbon plugin from `/carbon/plugin.cs`
///   into the Carbon plugins directory, with `UNIX_DOMAIN_SOCKET` substituted
///   into the plugin source.
/// - The function starts `RustDedicated` through Carbon, with output sent
///   directly to standard output and standard error, and waits for it to
///   stop.
/// - The function stops with the same exit code as `RustDedicated`.
///
/// Issuing commands to a running game server via the RCON WebSocket API is
/// out of scope for this function. The managing web server does that.
pub fn launch_game_server(config: &GameServerConfig) -> std::process::ExitCode {
    let steamcmd_present = std::process::Command::new("which")
        .arg("steamcmd")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);

    if !steamcmd_present {
        std::eprintln!("`steamcmd` was not found in `PATH`");
        return std::process::ExitCode::from(10);
    }

    let steamcmd_status = std::process::Command::new("steamcmd")
        .arg("+force_install_dir")
        .arg(&config.install_dir)
        .arg("+login")
        .arg("anonymous")
        .arg("+app_update")
        .arg("258550")
        .arg("validate")
        .arg("+quit")
        .status();

    match steamcmd_status {
        Ok(status) if status.success() => {}
        Ok(status) => {
            std::eprintln!("`steamcmd` exited with {status}");
            return std::process::ExitCode::from(11);
        }
        Err(err) => {
            std::eprintln!("failed to run `steamcmd`: {err}");
            return std::process::ExitCode::from(11);
        }
    }

    let async_runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(async_runtime) => async_runtime,
        Err(err) => {
            std::eprintln!("failed to start async runtime: {err}");
            return std::process::ExitCode::from(12);
        }
    };

    let install_dir = config.install_dir.clone();
    let carbon_install_result: Result<(), String> = async_runtime.block_on(async move {
        let response = reqwest::get(
            "https://github.com/CarbonCommunity/Carbon/releases/download/production_build/Carbon.Linux.Minimal.tar.gz",
        )
        .await
        .map_err(|err| err.to_string())?;

        let archive_bytes = response.bytes().await.map_err(|err| err.to_string())?;

        let gzip_decoder = flate2::read::GzDecoder::new(std::io::Cursor::new(archive_bytes));
        tar::Archive::new(gzip_decoder)
            .unpack(&install_dir)
            .map_err(|err| err.to_string())
    });

    if let Err(err) = carbon_install_result {
        std::eprintln!("failed to install Carbon: {err}");
        return std::process::ExitCode::from(13);
    }

    let plugin_source = std::include_str!("../../carbon/plugin.cs")
        .replace("/tmp/rustctl.sock", UNIX_DOMAIN_SOCKET);

    let carbon_plugins_dir = config.install_dir.join("carbon").join("plugins");

    if let Err(err) = std::fs::create_dir_all(&carbon_plugins_dir) {
        std::eprintln!("failed to create Carbon plugins directory: {err}");
        return std::process::ExitCode::from(14);
    }

    if let Err(err) = std::fs::write(carbon_plugins_dir.join("rustctl_sock.cs"), plugin_source) {
        std::eprintln!("failed to write Carbon plugin: {err}");
        return std::process::ExitCode::from(14);
    }

    match std::process::Command::new(config.install_dir.join("carbon.sh"))
        .current_dir(&config.install_dir)
        .status()
    {
        Ok(status) => std::process::ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(err) => {
            std::eprintln!("failed to launch `RustDedicated`: {err}");
            std::process::ExitCode::from(15)
        }
    }
}
