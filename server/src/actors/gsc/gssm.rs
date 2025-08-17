//! Game Server State Machine (GSSM).

pub enum GameServerStateMachine {
    Init {
        cfg_client: crate::storage::GameServerConfigurationShared,
        rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
        tx_aggregator: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
    },
    InstallingUpdates {
        cfg_client: crate::storage::GameServerConfigurationShared,
        rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
        tx_aggregator: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
    },
    InstalledAndConfigured {
        cfg_client: crate::storage::GameServerConfigurationShared,
        rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
        tx_aggregator: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

        game_meta: rustctl_common::snapshot::GameServerMetaExposed,
    },
    LaunchingGame {
        cfg_client: crate::storage::GameServerConfigurationShared,
        rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
        tx_aggregator: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

        game_meta: rustctl_common::snapshot::GameServerMetaExposed,
        process: tokio::process::Child,
        stdout: tokio::process::ChildStdout,
        stderr: tokio::process::ChildStderr,
    },
    GameRunningHealthy {
        cfg_client: crate::storage::GameServerConfigurationShared,
        rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
        tx_aggregator: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

        game_meta: rustctl_common::snapshot::GameServerMetaExposed,
        process: tokio::process::Child,
    },
    SavingAndClosingGame {
        cfg_client: crate::storage::GameServerConfigurationShared,
        rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
        tx_aggregator: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

        process: tokio::process::Child,
    },
    GameClosedManually {
        cfg_client: crate::storage::GameServerConfigurationShared,
        rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
        tx_aggregator: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
    },
    GameTerminatedUnexpectedly {
        cfg_client: crate::storage::GameServerConfigurationShared,
        rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
        tx_aggregator: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
    },
}

impl GameServerStateMachine {
    pub fn init(
        cfg_client: crate::storage::GameServerConfigurationShared,
        rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
        tx_aggregator: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,
    ) -> Self {
        Self::Init {
            cfg_client,
            rx_command,
            tx_aggregator,
            tx_activate,
        }
    }

    async fn request_termination(tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>) -> () {
        if let Err(err) = tx_activate
            .send(crate::actors::terminator::Activator::GameServerStateMachine)
            .await
        {
            log::error!("Failed to request graceful shutdown: {err}");
        }
    }

    pub async fn loop_transitions(mut self) -> () {
        'loop_transitions: loop {
            self = match self {
                Self::Init {
                    cfg_client,
                    rx_command,
                    tx_aggregator,
                    tx_activate,
                } => Self::InstallingUpdates {
                    cfg_client,
                    rx_command,
                    tx_aggregator,
                    tx_activate,
                },

                /*
                 * Install or update `RustDedicated` using `steamcmd`.
                 */
                Self::InstallingUpdates {
                    cfg_client,
                    rx_command,
                    tx_aggregator,
                    tx_activate,
                } => {
                    let config = cfg_client.get_config().await;

                    let buildid_before: Option<u32> = {
                        if let Ok(contents) = tokio::fs::read_to_string(config.game_manifest).await {
                            extract_buildid_from_buf(&contents)
                        } else {
                            None
                        }
                    };

                    let mut command = tokio::process::Command::new(config.installer_exe);
                    command.current_dir(config.game_server_root);
                    command.args(config.get_installer_args());
                    command.stdout(std::process::Stdio::null());
                    command.stderr(std::process::Stdio::null());

                    let process: tokio::process::Child = match command.spawn() {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!(
                                "Failed to spawn game server installer ({path}): {err_fmt}",
                                path = config.installer_exe,
                                err_fmt = crate::util::fmt_source_tree(&err),
                            );
                            Self::request_termination(tx_activate).await;
                            break 'loop_transitions;
                        }
                    };

                    let _output: std::process::Output = process.wait_with_output().await.unwrap();

                    let buildid_after: Option<u32> = {
                        if let Ok(contents) = tokio::fs::read_to_string(config.game_manifest).await {
                            extract_buildid_from_buf(&contents)
                        } else {
                            None
                        }
                    };

                    let buildid: u32 = match (buildid_before, buildid_after) {
                        (_, None) => {
                            log::error!(
                                "Installing game server failed: Could not extract buildid from game server app manifest after installation: {path}",
                                path = config.game_manifest
                            );
                            Self::request_termination(tx_activate).await;
                            break;
                        }
                        (None, Some(buildid)) => {
                            log::info!("Installed game server: buildid {buildid}");
                            buildid
                        }
                        (Some(buildid_before), Some(buildid_after)) => {
                            if buildid_before == buildid_after {
                                log::info!("Installation checked: Game server is up to date: buildid {buildid_after}");
                            } else {
                                log::info!("Updated game server: From buildid {buildid_before} to {buildid_after}");
                            }
                            buildid_after
                        }
                    };

                    Self::InstalledAndConfigured {
                        game_meta: rustctl_common::snapshot::GameServerMetaExposed { buildid },
                        cfg_client,
                        rx_command,
                        tx_aggregator,
                        tx_activate,
                    }
                }

                Self::InstalledAndConfigured {
                    cfg_client,
                    rx_command,
                    tx_aggregator,
                    tx_activate,
                    game_meta,
                } => {
                    let cfg = cfg_client.get_config().await;
                    let mut command = tokio::process::Command::new(cfg.game_server_exe);
                    command.current_dir(cfg.game_server_root);
                    command.args(cfg.get_game_args());
                    command.env("LD_LIBRARY_PATH", cfg.game_server_libs);
                    command.stdout(std::process::Stdio::piped());
                    command.stderr(std::process::Stdio::piped());

                    let mut process: tokio::process::Child = match command.spawn() {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!(
                                "Failed to spawn game server ({path}): {err_fmt}",
                                path = cfg.game_server_exe,
                                err_fmt = crate::util::fmt_source_tree(&err),
                            );
                            Self::request_termination(tx_activate).await;
                            break 'loop_transitions;
                        }
                    };

                    let stdout: tokio::process::ChildStdout = process.stdout.take().unwrap();
                    let stderr: tokio::process::ChildStderr = process.stderr.take().unwrap();

                    Self::LaunchingGame {
                        game_meta,
                        process,
                        stdout,
                        stderr,
                        cfg_client,
                        rx_command,
                        tx_aggregator,
                        tx_activate,
                    }
                }

                Self::LaunchingGame {
                    game_meta,
                    process,
                    stdout,
                    stderr,
                    cfg_client,
                    rx_command,
                    tx_aggregator,
                    tx_activate,
                } => {
                    let timeout = std::time::Duration::from_secs(60 * 30); // 30 minutes
                    let mut stdout_reader = tokio::io::BufReader::new(stdout);
                    let mut stderr_reader = tokio::io::BufReader::new(stderr);

                    // channel for signaling readiness from coroutine
                    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<()>();

                    let _read_stdout = tokio::spawn(async move {
                        let mut line = String::new();
                        let mut tx = Some(ready_tx);

                        loop {
                            line.clear();
                            match tokio::io::AsyncBufReadExt::read_line(&mut stdout_reader, &mut line).await {
                                Ok(0) => {
                                    log::debug!("EOF reached: game server STDOUT");
                                    break;
                                }
                                Ok(_) => {
                                    let trimmed_line = line.trim_end();
                                    log::debug!(target: crate::init::LOG_TARGET_GAME, "{trimmed_line}");
                                    if trimmed_line.contains("SteamServer Connected") {
                                        if let Some(sender) = tx.take() {
                                            let _ = sender.send(());
                                        }
                                    }
                                }
                                Err(err) => {
                                    log::error!(
                                        "Failed to read line from STDOUT: {err_fmt}",
                                        err_fmt = crate::util::fmt_source_tree(&err)
                                    );
                                    break;
                                }
                            }
                        }
                    });

                    let _read_stderr = tokio::spawn(async move {
                        let mut line = String::new();

                        loop {
                            line.clear();
                            match tokio::io::AsyncBufReadExt::read_line(&mut stderr_reader, &mut line).await {
                                Ok(0) => {
                                    log::debug!("EOF reached: game server STDERR");
                                    break;
                                }
                                Ok(_) => {
                                    let trimmed_line = line.trim_end();
                                    log::debug!(target: crate::init::LOG_TARGET_GAME, "{trimmed_line}");
                                }
                                Err(err) => {
                                    log::error!(
                                        "Failed to read line from STDERR: {err_fmt}",
                                        err_fmt = crate::util::fmt_source_tree(&err)
                                    );
                                    break;
                                }
                            }
                        }
                    });

                    match tokio::time::timeout(timeout, ready_rx).await {
                        Ok(Ok(_)) => Self::GameRunningHealthy {
                            process,
                            game_meta,
                            cfg_client,
                            rx_command,
                            tx_aggregator,
                            tx_activate,
                        },
                        Ok(Err(err)) => {
                            log::error!(
                                "Readiness signaling channel got teared down while waiting for the signal: {err_fmt}",
                                err_fmt = crate::util::fmt_source_tree(&err)
                            );
                            Self::request_termination(tx_activate).await;
                            break 'loop_transitions;
                        }
                        Err(err) => {
                            log::error!(
                                "Game server did not indicate its readiness within timeout of {timeout_secs} seconds: {err_fmt}",
                                timeout_secs = timeout.as_secs(),
                                err_fmt = crate::util::fmt_source_tree(&err)
                            );
                            Self::request_termination(tx_activate).await;
                            break 'loop_transitions;
                        }
                    }
                }

                Self::GameRunningHealthy {
                    game_meta,
                    mut process,
                    cfg_client,
                    mut rx_command,
                    tx_aggregator,
                    tx_activate,
                } => {
                    let event: GameCtlEvent = tokio::select! {
                        msg = rx_command.recv() => {
                            match msg {
                                Some(message) => GameCtlEvent::MessageReceived { message },
                                None => {
                                    log::error!("Channel for receiving commands closed while game server state machine is still working");
                                    Self::request_termination(tx_activate).await;
                                    break 'loop_transitions;
                                },
                            }
                        }
                        output = process.wait() => {
                            let exit_status: std::process::ExitStatus = match output {
                                Ok(n) => n,
                                Err(err) => {
                                    log::error!("Failed to run game server to termination: {err_fmt}", err_fmt = crate::util::fmt_source_tree(&err));
                                    Self::request_termination(tx_activate).await;
                                    break 'loop_transitions;
                                },
                            };
                            GameCtlEvent::GameProcessTerminated { exit_status }
                        }
                    };

                    match event {
                        GameCtlEvent::MessageReceived { message } => {
                            let command: rustctl_common::command::DownstreamClientMessage = message;
                            match command {
                                rustctl_common::command::DownstreamClientMessage::ServerSaveAndClose => {
                                    let signal = nix::sys::signal::Signal::SIGINT;
                                    let pid = match send_signal(&process, signal).await {
                                        Ok(n) => n,
                                        Err(err) => {
                                            log::error!(
                                                "Failed to send signal to game server: {err_fmt}",
                                                err_fmt = crate::util::fmt_source_tree(&err)
                                            );
                                            Self::request_termination(tx_activate).await;
                                            break 'loop_transitions;
                                        }
                                    };
                                    log::info!("Sent signal to game server process: {signal}: PID {pid}");
                                    Self::SavingAndClosingGame {
                                        process,
                                        cfg_client,
                                        rx_command,
                                        tx_aggregator,
                                        tx_activate,
                                    }
                                }
                                _ => {
                                    log::error!("Ignoring unexpected command: {command:?} for current state");
                                    Self::GameRunningHealthy {
                                        game_meta,
                                        process,
                                        cfg_client,
                                        rx_command,
                                        tx_aggregator,
                                        tx_activate,
                                    }
                                }
                            }
                        }
                        GameCtlEvent::GameProcessTerminated { exit_status } => {
                            let _exit_status: std::process::ExitStatus = exit_status;
                            Self::GameTerminatedUnexpectedly {
                                cfg_client,
                                rx_command,
                                tx_aggregator,
                                tx_activate,
                            }
                        }
                    }
                }

                Self::SavingAndClosingGame {
                    mut process,
                    cfg_client,
                    rx_command,
                    tx_aggregator,
                    tx_activate,
                } => {
                    match process.wait().await {
                        Ok(status) => {
                            log::info!("game server process exited with {status}");
                        }
                        Err(err) => {
                            log::error!(
                                "Waiting for game server process to terminate failed: {err_fmt}",
                                err_fmt = crate::util::fmt_source_tree(&err),
                            );
                            Self::request_termination(tx_activate).await;
                            break 'loop_transitions;
                        }
                    };

                    Self::GameClosedManually {
                        cfg_client,
                        rx_command,
                        tx_aggregator,
                        tx_activate,
                    }
                }

                Self::GameClosedManually {
                    cfg_client,
                    mut rx_command,
                    tx_aggregator,
                    tx_activate,
                } => {
                    let msg = rx_command.recv().await;
                    if let Some(command) = msg {
                        let command: rustctl_common::command::DownstreamClientMessage = command;
                        match command {
                            rustctl_common::command::DownstreamClientMessage::ServerInstallOrUpdateAndStart => {
                                Self::InstallingUpdates {
                                    cfg_client,
                                    rx_command,
                                    tx_aggregator,
                                    tx_activate,
                                }
                            }
                            _ => {
                                log::error!("Ignoring unexpected command: {command:?} for current state");
                                Self::GameClosedManually {
                                    cfg_client,
                                    rx_command,
                                    tx_aggregator,
                                    tx_activate,
                                }
                            }
                        }
                    } else {
                        Self::GameClosedManually {
                            cfg_client,
                            rx_command,
                            tx_aggregator,
                            tx_activate,
                        }
                    }
                }

                Self::GameTerminatedUnexpectedly {
                    cfg_client,
                    rx_command,
                    tx_aggregator,
                    tx_activate,
                } => Self::InstallingUpdates {
                    cfg_client,
                    rx_command,
                    tx_aggregator,
                    tx_activate,
                },
            };
        }
    }
}

impl std::fmt::Display for GameServerStateMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameServerStateMachine::Init { .. } => write!(f, "Init"),
            GameServerStateMachine::InstallingUpdates { .. } => write!(f, "Preparing"),
            GameServerStateMachine::InstalledAndConfigured { .. } => {
                write!(f, "InstalledAndConfigured")
            }
            GameServerStateMachine::LaunchingGame { .. } => write!(f, "Launching"),
            GameServerStateMachine::GameRunningHealthy { .. } => write!(f, "RunningHealthy"),
            GameServerStateMachine::SavingAndClosingGame { .. } => write!(f, "SavingAndClosing"),
            GameServerStateMachine::GameClosedManually { .. } => write!(f, "ClosedManually"),
            GameServerStateMachine::GameTerminatedUnexpectedly { .. } => {
                write!(f, "TerminatedUnexpectedly")
            }
        }
    }
}

fn extract_buildid_from_buf(buf: &str) -> Option<u32> {
    let vdf: keyvalues_parser::Vdf = match keyvalues_parser::Vdf::parse(buf) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let root: &keyvalues_parser::Obj = vdf.value.get_obj()?;

    let buildid_str: &str = match root.get("buildid") {
        Some(values) => {
            if values.len() != 1 {
                return None;
            }
            values[0].get_str()?
        }
        None => return None,
    };

    buildid_str.parse::<u32>().ok()
}

enum GameCtlEvent {
    MessageReceived {
        message: rustctl_common::command::DownstreamClientMessage,
    },

    GameProcessTerminated {
        exit_status: std::process::ExitStatus,
    },
}

async fn send_signal(
    child: &tokio::process::Child,
    signal: nix::sys::signal::Signal,
) -> Result<nix::unistd::Pid, ErrorSendingSignal> {
    let pid: u32 = match child.id() {
        Some(n) => n,
        None => return Err(ErrorSendingSignal::NoPid),
    };
    let pid: i32 = match pid.try_into() {
        Ok(n) => n,
        Err(source) => {
            return Err(ErrorSendingSignal::InvalidPid {
                source,
                invalid_pid: pid,
            });
        }
    };
    let pid: nix::unistd::Pid = nix::unistd::Pid::from_raw(pid);
    if let Err(source) = nix::sys::signal::kill(pid, signal) {
        Err(ErrorSendingSignal::SendFailed { source })
    } else {
        Ok(pid)
    }
}

#[derive(Debug)]
enum ErrorSendingSignal {
    NoPid,
    InvalidPid {
        source: std::num::TryFromIntError,
        invalid_pid: u32,
    },
    SendFailed {
        source: nix::errno::Errno,
    },
}

impl std::error::Error for ErrorSendingSignal {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ErrorSendingSignal::NoPid => None,
            ErrorSendingSignal::InvalidPid { source, .. } => Some(source),
            ErrorSendingSignal::SendFailed { source } => Some(source),
        }
    }
}

impl std::fmt::Display for ErrorSendingSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSendingSignal::NoPid => write!(f, "no PID"),
            ErrorSendingSignal::InvalidPid { source: _, invalid_pid } => write!(f, "invalid PID: {invalid_pid}"),
            ErrorSendingSignal::SendFailed { source: _ } => write!(f, "send failed"),
        }
    }
}
