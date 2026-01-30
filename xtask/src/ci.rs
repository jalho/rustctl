pub fn check_format_lint() -> Result<(), std::process::ExitCode> {
    check_errors()?;
    check_format()?;
    check_lint()?;
    check_web_bundle()?;
    Ok(())
}

fn check_errors() -> Result<(), std::process::ExitCode> {
    let mut command = std::process::Command::new("cargo");
    command.args(vec!["check", "--workspace"]);

    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());

    log::info!("Checking for errors...");
    let status: std::process::ExitStatus = match command.status() {
        Ok(n) => n,
        Err(err) => {
            log::error!(
                "Failed to check errors: {command}: {err}",
                command = command.format()
            );
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    match status.success() {
        true => {
            log::info!("Error check passed: {command}", command = command.format());
            Ok(())
        }
        false => match status.code() {
            Some(_code) => {
                log::error!(
                    "Error check not passed: {command}",
                    command = command.format()
                );
                Err(std::process::ExitCode::FAILURE)
            }
            None => {
                log::error!(
                    "Failed to check errors: {command}: No exit code",
                    command = command.format()
                );
                Err(std::process::ExitCode::FAILURE)
            }
        },
    }
}

fn check_format() -> Result<(), std::process::ExitCode> {
    let mut command = std::process::Command::new("cargo");
    command.args(vec!["fmt", "--check"]);

    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());

    log::info!("Checking formatting...");
    let status: std::process::ExitStatus = match command.status() {
        Ok(n) => n,
        Err(err) => {
            log::error!(
                "Failed to check formatting: {command}: {err}",
                command = command.format()
            );
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    match status.success() {
        true => {
            log::info!("Format check passed: {command}", command = command.format());
            Ok(())
        }
        false => match status.code() {
            Some(_code) => {
                log::error!(
                    "Format check not passed: {command}",
                    command = command.format()
                );
                Err(std::process::ExitCode::FAILURE)
            }
            None => {
                log::error!(
                    "Failed to check formatting: {command}: No exit code",
                    command = command.format()
                );
                Err(std::process::ExitCode::FAILURE)
            }
        },
    }
}

fn check_lint() -> Result<(), std::process::ExitCode> {
    let mut command = std::process::Command::new("cargo");
    command.args(vec!["clippy", "--workspace", "--", "--deny", "warnings"]);

    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());

    log::info!("Checking lint...");
    let status: std::process::ExitStatus = match command.status() {
        Ok(n) => n,
        Err(err) => {
            log::error!(
                "Failed to check lints: {command}: {err}",
                command = command.format()
            );
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    match status.success() {
        true => {
            log::info!("Lint check passed: {command}", command = command.format());
            Ok(())
        }
        false => match status.code() {
            Some(_code) => {
                log::error!(
                    "Lint check not passed: {command}",
                    command = command.format()
                );
                Err(std::process::ExitCode::FAILURE)
            }
            None => {
                log::error!(
                    "Failed to check lints: {command}: No exit code",
                    command = command.format()
                );
                Err(std::process::ExitCode::FAILURE)
            }
        },
    }
}

fn check_web_bundle() -> Result<(), std::process::ExitCode> {
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

    log::info!("Checking release bundle for web...");
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
