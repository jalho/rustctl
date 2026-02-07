pub fn build_release_deb() -> Result<(), std::process::ExitCode> {
    build_release_backend()?;
    bundle_package_apt()?;
    verify_deb_package()?;
    Ok(())
}

fn build_release_backend() -> Result<(), std::process::ExitCode> {
    let mut command = std::process::Command::new("cargo");
    command.args(vec![
        "build",
        "--release",
        "--bin",
        "backend",
        "--target",
        "x86_64-unknown-linux-musl",
    ]);

    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());

    log::info!("Building backend release...");
    let status: std::process::ExitStatus = match command.status() {
        Ok(n) => n,
        Err(err) => {
            log::error!(
                "Building backend release: {command}: {err}",
                command = command.format()
            );
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    match status.success() {
        true => {
            log::info!(
                "Building backend release succeeded: {command}",
                command = command.format()
            );
            Ok(())
        }
        false => match status.code() {
            Some(_code) => {
                log::error!(
                    "Building backend release failed: {command}",
                    command = command.format()
                );
                Err(std::process::ExitCode::FAILURE)
            }
            None => {
                log::error!(
                    "Building backend release failed: {command}: No exit code",
                    command = command.format()
                );
                Err(std::process::ExitCode::FAILURE)
            }
        },
    }
}

fn bundle_package_apt() -> Result<(), std::process::ExitCode> {
    let version = get_project_version()?;
    let staging_dir = "target/deb_staging";

    prepare_staging_dir(staging_dir)?;
    write_systemd_unit(staging_dir)?;
    write_control_file(staging_dir, &version)?;
    run_dpkg_build(staging_dir)?;

    Ok(())
}

fn get_project_version() -> Result<String, std::process::ExitCode> {
    log::info!("Extracting version from cargo metadata...");
    let mut command = std::process::Command::new("cargo");
    command.args(vec!["metadata", "--format-version", "1", "--no-deps"]);

    let output = match command.output() {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            log::error!(
                "Extracting version failed: {command}: Exit code {code:?}",
                command = command.format(),
                code = output.status.code()
            );
            return Err(std::process::ExitCode::FAILURE);
        }
        Err(err) => {
            log::error!(
                "Extracting version failed: {command}: {err}",
                command = command.format()
            );
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    let metadata: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(json) => json,
        Err(err) => {
            log::error!("Parsing cargo metadata failed: {err}");
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    let version = metadata["packages"]
        .as_array()
        .and_then(|packages| packages.iter().find(|p| p["name"] == "backend"))
        .and_then(|package| package["version"].as_str());

    match version {
        Some(v) => Ok(v.to_string()),
        None => {
            log::error!("Building package failed: 'backend' not found in metadata");
            Err(std::process::ExitCode::FAILURE)
        }
    }
}

fn prepare_staging_dir(path: &str) -> Result<(), std::process::ExitCode> {
    log::info!("Preparing staging directory...");

    if std::path::Path::new(path).exists()
        && let Err(err) = std::fs::remove_dir_all(path)
    {
        log::error!("Failed to remove old staging directory {path}: {err}");
        return Err(std::process::ExitCode::FAILURE);
    }

    let dirs = [
        format!("{path}/usr/bin"),
        format!("{path}/lib/systemd/system"),
        format!("{path}/DEBIAN"),
    ];

    for dir in dirs {
        if let Err(err) = std::fs::create_dir_all(&dir) {
            log::error!("Failed to create directory {dir}: {err}");
            return Err(std::process::ExitCode::FAILURE);
        }
    }

    let src = "target/x86_64-unknown-linux-musl/release/backend";
    let dest = format!("{path}/usr/bin/rustctl");

    if let Err(err) = std::fs::copy(src, &dest) {
        log::error!("Failed to copy binary from {src} to {dest}: {err}");
        return Err(std::process::ExitCode::FAILURE);
    }

    Ok(())
}

fn write_systemd_unit(path: &str) -> Result<(), std::process::ExitCode> {
    log::info!("Writing systemd unit...");

    let content = r#"[Unit]
Description=rustctl
After=network.target

[Service]
User=rustctl
ExecStart=/usr/bin/rustctl service
Restart=always

[Install]
WantedBy=multi-user.target
"#;

    let file_path = format!("{path}/lib/systemd/system/rustctl.service");
    if let Err(err) = std::fs::write(&file_path, content) {
        log::error!("Failed to write systemd unit to {file_path}: {err}");
        return Err(std::process::ExitCode::FAILURE);
    }

    Ok(())
}

fn write_control_file(path: &str, version: &str) -> Result<(), std::process::ExitCode> {
    log::info!("Writing control file...");

    let content = format!(
        r#"Package: rustctl
Version: {version}
Architecture: amd64
Maintainer: TODO
Description: TODO
"#
    );

    let file_path = format!("{path}/DEBIAN/control");
    if let Err(err) = std::fs::write(&file_path, content) {
        log::error!("Failed to write control file to {file_path}: {err}");
        return Err(std::process::ExitCode::FAILURE);
    }

    Ok(())
}

fn run_dpkg_build(path: &str) -> Result<(), std::process::ExitCode> {
    let mut command = std::process::Command::new("dpkg-deb");
    command.args(vec!["--build", path, "target/rustctl.deb"]);

    log::info!("Building deb package...");
    let status = execute_step(command);
    let _ = std::fs::remove_dir_all(path);
    status
}

fn verify_deb_package() -> Result<(), std::process::ExitCode> {
    log::info!("Verifying .deb package content...");
    let path = "target/rustctl.deb";

    if !std::path::Path::new(path).exists() {
        log::error!("Verification failed: {path} not found");
        return Err(std::process::ExitCode::FAILURE);
    }

    let mut command = std::process::Command::new("dpkg");
    command.args(vec!["-c", path]);

    let output = match command.output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
        Ok(out) => {
            log::error!(
                "Verification failed: {command} exited with {code:?}",
                command = command.format(),
                code = out.status.code()
            );
            return Err(std::process::ExitCode::FAILURE);
        }
        Err(err) => {
            log::error!(
                "Verification failed: {command}: {err}",
                command = command.format()
            );
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    let expected_files = ["./usr/bin/rustctl", "./lib/systemd/system/rustctl.service"];

    for file in expected_files {
        if !output.contains(file) {
            log::error!("Verification failed: {path} is missing {file}");
            return Err(std::process::ExitCode::FAILURE);
        }
    }

    log::info!("Verification succeeded: {path} contains all required assets");
    Ok(())
}

fn execute_step(mut command: std::process::Command) -> Result<(), std::process::ExitCode> {
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());

    let status: std::process::ExitStatus = match command.status() {
        Ok(n) => n,
        Err(err) => {
            log::error!(
                "Command execution failed: {command}: {err}",
                command = command.format()
            );
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    match status.success() {
        true => Ok(()),
        false => {
            log::error!(
                "Command execution failed: {command}",
                command = command.format()
            );
            Err(std::process::ExitCode::FAILURE)
        }
    }
}

trait Display {
    fn format(&self) -> String;
}

impl Display for std::process::Command {
    fn format(&self) -> String {
        format!(
            "{} {}",
            self.get_program().to_string_lossy(),
            self.get_args()
                .map(|n| n.to_string_lossy().to_string())
                .collect::<Vec<String>>()
                .join(" "),
        )
    }
}
