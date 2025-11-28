pub fn make_package() -> Result<(), std::process::ExitCode> {
    log::info!("Building server...");
    let status = std::process::Command::new("cargo")
        .args(["build", "--release", "--target", "x86_64-unknown-linux-musl"])
        .current_dir("./server")
        .status()
        .map_err(|e| {
            log::error!("Failed to execute cargo build: {}", e);
            std::process::ExitCode::FAILURE
        })?;

    if !status.success() {
        log::error!("Server build failed");
        return Err(std::process::ExitCode::FAILURE);
    }

    log::info!("Building web client...");
    let status = std::process::Command::new("dx")
        .args(["bundle", "--platform", "web"])
        .current_dir("./clients/web")
        .status()
        .map_err(|e| {
            log::error!("Failed to execute dx bundle: {}", e);
            std::process::ExitCode::FAILURE
        })?;

    if !status.success() {
        log::error!("Web build failed");
        return Err(std::process::ExitCode::FAILURE);
    }

    let package_name = "rustctl";
    let version = read_version_from_cargo_toml()?;
    let arch = "amd64";
    let deb_dir = "target/debian";
    let package_dir = format!("{}/{}_{}_{}", deb_dir, package_name, version, arch);

    log::info!("Setting up package directory...");
    let _ = std::fs::remove_dir_all(deb_dir);
    std::fs::create_dir_all(format!("{}/DEBIAN", package_dir)).map_err(|e| {
        log::error!("Failed to create DEBIAN dir: {}", e);
        std::process::ExitCode::FAILURE
    })?;
    std::fs::create_dir_all(format!("{}/usr/bin", package_dir)).map_err(|e| {
        log::error!("Failed to create usr/bin dir: {}", e);
        std::process::ExitCode::FAILURE
    })?;
    std::fs::create_dir_all(format!("{}/var/lib/rustctl/web", package_dir)).map_err(|e| {
        log::error!("Failed to create var/lib/rustctl/web dir: {}", e);
        std::process::ExitCode::FAILURE
    })?;

    log::info!("Copying binary...");
    std::fs::copy(
        "./target/x86_64-unknown-linux-musl/release/rustctl-backend",
        format!("{}/usr/bin/rustctl-backend", package_dir),
    )
    .map_err(|e| {
        log::error!("Failed to copy binary: {}", e);
        std::process::ExitCode::FAILURE
    })?;

    log::info!("Copying web assets...");
    copy_dir_recursive(
        std::path::Path::new("./target/dx/rustctl-web/release/web/public"),
        std::path::Path::new(&format!("{}/var/lib/rustctl/web", package_dir)),
    )?;

    log::info!("Writing control file...");
    let control_content = format!(
        "Package: {}\nVersion: {}\nSection: base\nPriority: optional\nArchitecture: {}\nMaintainer: TODO <todo@todo>\nDescription: rustctl\n Tooling for running a Rust (the game) server and an integrated web service.\n",
        package_name, version, arch
    );
    std::fs::write(format!("{}/DEBIAN/control", package_dir), control_content).map_err(|e| {
        log::error!("Failed to write control file: {}", e);
        std::process::ExitCode::FAILURE
    })?;

    log::info!("Building .deb package...");
    let status = std::process::Command::new("dpkg-deb")
        .args(["--build", &package_dir])
        .status()
        .map_err(|e| {
            log::error!("Failed to execute dpkg-deb: {}", e);
            std::process::ExitCode::FAILURE
        })?;

    if !status.success() {
        log::error!("dpkg-deb failed");
        return Err(std::process::ExitCode::FAILURE);
    }

    let output = std::process::Command::new("file")
        .arg(format!("{}.deb", package_dir))
        .output()
        .map_err(|e| {
            log::error!("Failed to execute file command: {}", e);
            std::process::ExitCode::FAILURE
        })?;

    log::info!("{}", String::from_utf8_lossy(&output.stdout));

    log::info!("Package built: {}.deb", package_dir);

    Ok(())
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), std::process::ExitCode> {
    for entry in std::fs::read_dir(src).map_err(|e| {
        log::error!("Failed to read source directory: {}", e);
        std::process::ExitCode::FAILURE
    })? {
        let entry = entry.map_err(|e| {
            log::error!("Failed to read directory entry: {}", e);
            std::process::ExitCode::FAILURE
        })?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            std::fs::create_dir_all(&dest_path).map_err(|e| {
                log::error!("Failed to create directory: {}", e);
                std::process::ExitCode::FAILURE
            })?;
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            std::fs::copy(&path, &dest_path).map_err(|e| {
                log::error!("Failed to copy file: {}", e);
                std::process::ExitCode::FAILURE
            })?;
        }
    }
    Ok(())
}

fn read_version_from_cargo_toml() -> Result<String, std::process::ExitCode> {
    let content = std::fs::read_to_string("./server/Cargo.toml").map_err(|e| {
        log::error!("Failed to read Cargo.toml: {}", e);
        std::process::ExitCode::FAILURE
    })?;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version")
            && let Some(value) = trimmed.split('=').nth(1)
        {
            let version = value.trim().trim_matches('"').to_string();
            return Ok(version);
        }
    }

    log::error!("Version not found in Cargo.toml");
    Err(std::process::ExitCode::FAILURE)
}
