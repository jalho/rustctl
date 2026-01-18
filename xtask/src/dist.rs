pub fn make_release() -> Result<(), std::process::ExitCode> {
    make_release_frontend_web()?;
    make_release_backend()?;
    Ok(())
}

fn make_release_frontend_web() -> Result<(), std::process::ExitCode> {
    let mut command = std::process::Command::new("dx");
    command.args(vec![
        "bundle",
        "--web",
        "--release",
        "--package",
        "frontend",
    ]);

    let status: std::process::ExitStatus = match command.status() {
        Ok(n) => n,
        Err(err) => {
            log::error!("Failed to bundle web release: {command:?}: {err}");
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    match status.success() {
        true => {
            log::info!("Making release bundle for web succeeded: {command:?}");
            Ok(())
        }
        false => match status.code() {
            Some(code) => {
                log::error!("Making release bundle for web failed: {command:?}: {code:?}");
                Err(std::process::ExitCode::FAILURE)
            }
            None => {
                log::error!("Failed to make release bundle for web: {command:?}: No exit code");
                Err(std::process::ExitCode::FAILURE)
            }
        },
    }
}

fn make_release_backend() -> Result<(), std::process::ExitCode> {
    let mut command = std::process::Command::new("cargo");
    command.args(vec![
        "build",
        "--release",
        "--bin",
        "backend",
        "--target",
        "x86_64-unknown-linux-musl",
    ]);

    let status: std::process::ExitStatus = match command.status() {
        Ok(n) => n,
        Err(err) => {
            log::error!("Building backend release: {command:?}: {err}");
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    match status.success() {
        true => {
            log::info!("Building backend release succeeded: {command:?}");
            Ok(())
        }
        false => match status.code() {
            Some(code) => {
                log::error!("Building backend release failed: {command:?}: {code:?}");
                Err(std::process::ExitCode::FAILURE)
            }
            None => {
                log::error!("Building backend release failed: {command:?}: No exit code");
                Err(std::process::ExitCode::FAILURE)
            }
        },
    }
}
