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

    crate::execute_step(command)
}

fn copy_to_server(local: &str, host: &str, remote: &str) -> Result<(), std::process::ExitCode> {
    log::info!("Transferring package...");
    let mut command = std::process::Command::new("scp");
    command.args(vec![local, &format!("{host}:{remote}")]);

    crate::execute_step(command)
}

fn install_on_server(host: &str, remote: &str) -> Result<(), std::process::ExitCode> {
    log::info!("Installing and starting service...");
    let mut command = std::process::Command::new("ssh");
    command.args(vec![
        "-t",
        host,
        &format!(
            "sudo systemctl stop rustctl || true && \
             sudo dpkg -i {remote} && \
             sudo systemctl daemon-reload && \
             sudo systemctl enable --now rustctl && \
             rm {remote}"
        ),
    ]);

    crate::execute_step(command)
}
