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

    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());

    log::info!("Making release bundle for web...");
    let status: std::process::ExitStatus = match command.status() {
        Ok(n) => n,
        Err(err) => {
            log::error!(
                "Failed to bundle web release: {command}: {err}",
                command = command.format()
            );
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    match status.success() {
        true => {
            log::info!(
                "Making release bundle for web succeeded: {command}",
                command = command.format()
            );
            Ok(())
        }
        false => match status.code() {
            Some(_code) => {
                log::error!(
                    "Making release bundle for web failed: {command}",
                    command = command.format()
                );
                Err(std::process::ExitCode::FAILURE)
            }
            None => {
                log::error!(
                    "Failed to make release bundle for web: {command}: No exit code",
                    command = command.format()
                );
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
