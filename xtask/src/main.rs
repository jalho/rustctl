mod ci;
mod deploy;
mod dist;
mod init;

fn main() -> Result<(), std::process::ExitCode> {
    let cli: init::Cli = init::Cli::get();
    let _logg: log4rs::Handle = init::init_logger()?;

    match cli.command {
        None | Some(init::Command::Ci) => {
            ci::check_format_lint()?;
        }
        Some(init::Command::Dist) => {
            ci::check_format_lint()?;
            dist::build_release_deb()?;
        }
        Some(init::Command::Deploy) => {
            deploy::via_ssh()?;
        }
    }

    Ok(())
}

pub trait Display {
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

pub fn execute_step(mut command: std::process::Command) -> Result<(), std::process::ExitCode> {
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

    if status.success() {
        Ok(())
    } else {
        log::error!(
            "Command execution failed: {command}",
            command = command.format()
        );
        Err(std::process::ExitCode::FAILURE)
    }
}
