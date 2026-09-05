pub const UNIX_DOMAIN_SOCKET: &'static str = "/tmp/rustctl.sock";
pub const RCON_PORT: u16 = 28016;

pub struct GameServerConfig {
    pub install_dir: std::path::PathBuf,
    pub rcon_password: &'static str,
}

impl Default for GameServerConfig {
    fn default() -> Self {
        Self {
            install_dir: std::path::PathBuf::from("/srv/rustctl/game/RustDedicated"),
            rcon_password: "",
        }
    }
}

/// Update and launch game server (executable named `RustDedicated`) with a _Carbon_
/// (modding framework) plugin installed.
///
/// In more detail:
///
/// - The function checks that `steamcmd` is present. If `steamcmd` is not
///   present, the function stops with an error code.
///
/// - The function installs or updates `RustDedicated` with `steamcmd`.
///
/// - The function downloads and unpacks the Carbon modding framework from
///   GitHub.
///
/// - The function writes the custom Carbon plugin (named `rustctl_sock`) into the
///   Carbon plugins directory, with [`UNIX_DOMAIN_SOCKET`] substituted into the
///   plugin source.
///
/// - The function starts `RustDedicated` through a generated startup script that
///   sets `LD_LIBRARY_PATH`, sources Carbon's `carbon/tools/environment.sh`, and
///   passes the RCON flags that the managing web server needs to connect (RCON
///   TCP port, RCON password), with output sent directly to standard output and
///   standard error, and waits for it to stop.
///
/// - The function stops with the same exit code as `RustDedicated`.
///
/// Issuing commands to a running game server via the RCON WebSocket API, like any
/// configuring of Carbon via RCON, is out of scope for this function. The managing
/// web server shall do that.
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

    let startup_script_path = config.install_dir.join("rustctl-run-with-carbon.sh");
    let startup_script_content = std::format!(
        "#!/bin/bash\nset -e\nexport LD_LIBRARY_PATH=\"{install_dir}\"\nsource \"{install_dir}/carbon/tools/environment.sh\"\nexec \"{install_dir}/RustDedicated\" \\\n    -batchmode \\\n    +rcon.port \"{rcon_port}\" \\\n    +rcon.web \"1\" \\\n    +rcon.password \"{rcon_password}\"\n",
        install_dir = config.install_dir.display(),
        rcon_port = RCON_PORT,
        rcon_password = config.rcon_password,
    );

    if let Err(err) = std::fs::write(&startup_script_path, startup_script_content) {
        std::eprintln!("failed to write game server startup script: {err}");
        return std::process::ExitCode::from(16);
    }

    let mut startup_script_permissions = match std::fs::metadata(&startup_script_path) {
        Ok(metadata) => metadata.permissions(),
        Err(err) => {
            std::eprintln!("failed to read game server startup script metadata: {err}");
            return std::process::ExitCode::from(16);
        }
    };
    {
        use std::os::unix::fs::PermissionsExt;
        startup_script_permissions.set_mode(0o755);
    }
    if let Err(err) = std::fs::set_permissions(&startup_script_path, startup_script_permissions) {
        std::eprintln!("failed to make game server startup script executable: {err}");
        return std::process::ExitCode::from(16);
    }

    match std::process::Command::new(&startup_script_path)
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
