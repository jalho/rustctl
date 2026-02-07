pub fn via_ssh() -> Result<(), std::process::ExitCode> {
    let host = "rustctl";
    let deb_path = "target/rustctl.deb";
    let remote_path = "/tmp/rustctl.deb";

    verify_ssh_connectivity(host)?;
    copy_to_server(deb_path, host, remote_path)?;
    install_on_server(host, remote_path)?;

    Ok(())
}

fn verify_ssh_connectivity(host: &str) -> Result<(), std::process::ExitCode> {
    log::info!("Verifying SSH connectivity to {host}...");

    let mut command = std::process::Command::new("ssh");
    command.args(vec![
        "-o",
        "ConnectTimeout=2",
        "-o",
        "BatchMode=yes",
        host,
        "lsb_release -a",
    ]);

    let output = match command.output() {
        Ok(out) if out.status.success() => out,
        Ok(out) => {
            log::error!(
                "SSH connectivity check failed: {command}: Exit code {code:?}",
                command = format_command(&command),
                code = out.status.code()
            );
            return Err(std::process::ExitCode::FAILURE);
        }
        Err(err) => {
            log::error!(
                "SSH connectivity check failed: {command}: {err}",
                command = format_command(&command)
            );
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    let info = String::from_utf8_lossy(&output.stdout);
    for line in info.lines() {
        log::info!("Remote system info: {line}");
    }

    Ok(())
}

fn copy_to_server(local: &str, host: &str, remote: &str) -> Result<(), std::process::ExitCode> {
    log::info!("Transferring {local} to {host}:{remote}...");

    let mut command = std::process::Command::new("scp");
    command.args(vec![local, &format!("{host}:{remote}")]);

    execute_deploy_step(command)
}

fn install_on_server(host: &str, remote: &str) -> Result<(), std::process::ExitCode> {
    log::info!("Installing and starting rustctl on {host}...");

    let mut command = std::process::Command::new("ssh");
    command.args(vec![
        "-t",
        host,
        &format!(
            "sudo dpkg -i {remote} && \
             sudo systemctl daemon-reload && \
             sudo systemctl enable --now rustctl && \
             rm {remote}"
        ),
    ]);

    execute_deploy_step(command)
}

fn execute_deploy_step(mut command: std::process::Command) -> Result<(), std::process::ExitCode> {
    let status: std::process::ExitStatus = match command.status() {
        Ok(n) => n,
        Err(err) => {
            log::error!(
                "Deployment step failed: {command}: {err}",
                command = format_command(&command)
            );
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    match status.success() {
        true => Ok(()),
        false => {
            log::error!(
                "Deployment step failed: {command}",
                command = format_command(&command)
            );
            Err(std::process::ExitCode::FAILURE)
        }
    }
}

fn format_command(command: &std::process::Command) -> String {
    format!(
        "{} {}",
        command.get_program().to_string_lossy(),
        command
            .get_args()
            .map(|n| n.to_string_lossy().to_string())
            .collect::<Vec<String>>()
            .join(" "),
    )
}
