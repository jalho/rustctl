use crate::Display;

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

    log::info!("Building backend release...");
    crate::execute_step(command)
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
    command.stderr(std::process::Stdio::null());

    let output = match command.output() {
        Ok(out) if out.status.success() => out,
        Ok(out) => {
            log::error!(
                "Version extraction failed: {command} (Exit: {code:?})",
                command = command.format(),
                code = out.status.code()
            );
            return Err(std::process::ExitCode::FAILURE);
        }
        Err(err) => {
            log::error!("Version extraction failed: {err}");
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|_| std::process::ExitCode::FAILURE)?;

    metadata["packages"]
        .as_array()
        .and_then(|pkgs| pkgs.iter().find(|p| p["name"] == "backend"))
        .and_then(|p| p["version"].as_str())
        .map(|v| v.to_string())
        .ok_or(std::process::ExitCode::FAILURE)
}

fn prepare_staging_dir(path: &str) -> Result<(), std::process::ExitCode> {
    log::info!("Preparing staging directory...");
    if std::path::Path::new(path).exists() {
        std::fs::remove_dir_all(path).map_err(|_| std::process::ExitCode::FAILURE)?;
    }

    let dirs = [
        format!("{path}/usr/bin"),
        format!("{path}/lib/systemd/system"),
        format!("{path}/DEBIAN"),
    ];

    for dir in dirs {
        std::fs::create_dir_all(&dir).map_err(|_| std::process::ExitCode::FAILURE)?;
    }

    std::fs::copy(
        "target/x86_64-unknown-linux-musl/release/backend",
        format!("{path}/usr/bin/rustctl"),
    )
    .map_err(|_| std::process::ExitCode::FAILURE)?;

    Ok(())
}

fn write_systemd_unit(path: &str) -> Result<(), std::process::ExitCode> {
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
    std::fs::write(
        format!("{path}/lib/systemd/system/rustctl.service"),
        content,
    )
    .map_err(|_| std::process::ExitCode::FAILURE)
}

fn write_control_file(path: &str, version: &str) -> Result<(), std::process::ExitCode> {
    let content = format!(
        "Package: rustctl\nVersion: {version}\nArchitecture: amd64\nMaintainer: admin\nDescription: rustctl\n"
    );
    std::fs::write(format!("{path}/DEBIAN/control"), content)
        .map_err(|_| std::process::ExitCode::FAILURE)
}

fn run_dpkg_build(path: &str) -> Result<(), std::process::ExitCode> {
    let mut command = std::process::Command::new("dpkg-deb");
    command.args(vec!["--build", path, "target/rustctl.deb"]);

    log::info!("Packaging .deb...");
    let res = crate::execute_step(command);
    let _ = std::fs::remove_dir_all(path);
    res
}

fn verify_deb_package() -> Result<(), std::process::ExitCode> {
    log::info!("Verifying package content...");
    let mut command = std::process::Command::new("dpkg");
    command.args(vec!["-c", "target/rustctl.deb"]);
    command.stderr(std::process::Stdio::null());

    let output = command
        .output()
        .map_err(|_| std::process::ExitCode::FAILURE)?;
    let list = String::from_utf8_lossy(&output.stdout);

    if list.contains("./usr/bin/rustctl") && list.contains("rustctl.service") {
        Ok(())
    } else {
        log::error!("Package verification failed: missing files");
        Err(std::process::ExitCode::FAILURE)
    }
}
