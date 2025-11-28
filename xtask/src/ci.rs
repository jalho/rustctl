pub fn check_format_lint() -> Result<(), std::process::ExitCode> {
    check_errors()?;
    check_format()?;
    check_lint()?;
    Ok(())
}

fn check_errors() -> Result<(), std::process::ExitCode> {
    let mut command = std::process::Command::new("cargo");
    command.args(vec!["check", "--workspace"]);

    let status: std::process::ExitStatus = match command.status() {
        Ok(n) => n,
        Err(err) => {
            log::error!("Failed to check errors: {command:?}: {err}");
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    match status.success() {
        true => {
            log::info!("Error check passed: {command:?}");
            Ok(())
        }
        false => match status.code() {
            Some(code) => {
                log::error!("Error check not passed: {command:?}: {code:?}");
                Err(std::process::ExitCode::FAILURE)
            }
            None => {
                log::error!("Failed to check errors: {command:?}: No exit code");
                Err(std::process::ExitCode::FAILURE)
            }
        },
    }
}

fn check_format() -> Result<(), std::process::ExitCode> {
    let mut command = std::process::Command::new("cargo");
    command.args(vec!["fmt", "--check"]);

    let status: std::process::ExitStatus = match command.status() {
        Ok(n) => n,
        Err(err) => {
            log::error!("Failed to check formatting: {command:?}: {err}");
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    match status.success() {
        true => {
            log::info!("Format check passed: {command:?}");
            Ok(())
        }
        false => match status.code() {
            Some(code) => {
                log::error!("Format check not passed: {command:?}: {code:?}");
                Err(std::process::ExitCode::FAILURE)
            }
            None => {
                log::error!("Failed to check formatting: {command:?}: No exit code");
                Err(std::process::ExitCode::FAILURE)
            }
        },
    }
}

fn check_lint() -> Result<(), std::process::ExitCode> {
    let mut command = std::process::Command::new("cargo");
    command.args(vec!["clippy", "--workspace", "--", "--deny", "warnings"]);

    let status: std::process::ExitStatus = match command.status() {
        Ok(n) => n,
        Err(err) => {
            log::error!("Failed to check lints: {command:?}: {err}");
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    match status.success() {
        true => {
            log::info!("Lint check passed: {command:?}");
            Ok(())
        }
        false => match status.code() {
            Some(code) => {
                log::error!("Lint check not passed: {command:?}: {code:?}");
                Err(std::process::ExitCode::FAILURE)
            }
            None => {
                log::error!("Failed to check lints: {command:?}: No exit code");
                Err(std::process::ExitCode::FAILURE)
            }
        },
    }
}
