//! Game Server State Machine (GSSM).

pub struct Context {
    pub tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

    pub cfg_client: crate::storage::ConfigurationClient,

    pub rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,

    /// "GSS" = "Game Server State"
    pub tx_agg_gss: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,

    pub tx_rconready: tokio::sync::mpsc::Sender<crate::actors::gsc::gssm::ReadyForRcon>,
}

pub enum GameServerStateMachine {
    Init {
        ctx: Context,
    },
    InstallingUpdates {
        ctx: Context,
    },
    InstalledAndConfigured {
        ctx: Context,
        game_meta: rustctl_common::snapshot::GameServerMetaExposed,
        startup_script: String,
    },
    LaunchingGame {
        ctx: Context,
        game_meta: rustctl_common::snapshot::GameServerMetaExposed,
        process: tokio::process::Child,
        stdout: tokio::process::ChildStdout,
        stderr: tokio::process::ChildStderr,
    },
    GameRunningHealthy {
        ctx: Context,
        game_meta: rustctl_common::snapshot::GameServerMetaExposed,
        process: tokio::process::Child,
    },
    SavingAndClosingGame {
        ctx: Context,
        process: tokio::process::Child,
    },
    GameClosedManually {
        ctx: Context,
    },
    GameTerminatedUnexpectedly {
        ctx: Context,
    },
}

impl GameServerStateMachine {
    pub fn init(
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

        cfg_client: crate::storage::ConfigurationClient,

        rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,

        tx_agg_gss: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,

        tx_rconready: tokio::sync::mpsc::Sender<crate::actors::gsc::gssm::ReadyForRcon>,
    ) -> Self {
        Self::Init {
            ctx: Context {
                tx_activate,

                cfg_client,

                rx_command,

                tx_agg_gss,

                tx_rconready,
            },
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
                Self::Init { ctx } => {
                    let config: crate::storage::Configuration = ctx.cfg_client.get_config().await;

                    let running_already: Vec<u32> = is_running_already(&config).await;
                    if !running_already.is_empty() {
                        log::error!(
                            "Running already -- Process(es) ({count} pcs) {pids} should be terminated!",
                            count = running_already.len(),
                            pids = running_already
                                .iter()
                                .map(|pid| pid.to_string())
                                .collect::<Vec<String>>()
                                .join(", "),
                        );
                        Self::request_termination(ctx.tx_activate.clone()).await;
                        break 'loop_transitions;
                    }

                    Self::InstallingUpdates { ctx }
                }

                /*
                 * Install or update `RustDedicated` using `steamcmd`.
                 */
                Self::InstallingUpdates { ctx } => {
                    let config: crate::storage::Configuration = ctx.cfg_client.get_config().await;

                    let buildid_before: Option<u32> = {
                        if let Ok(contents) = tokio::fs::read_to_string(config.fs.manifest_abs_utf8()).await {
                            extract_buildid_from_buf(&contents)
                        } else {
                            None
                        }
                    };

                    let buildid_after: u32 = match install_or_update_game_server(&config).await {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!("Installing game server failed: {err}");
                            Self::request_termination(ctx.tx_activate.clone()).await;
                            break 'loop_transitions;
                        }
                    };

                    match buildid_before {
                        None => {
                            log::info!("Installed game server: buildid {buildid_after}");
                        }
                        Some(buildid_before) => {
                            if buildid_before == buildid_after {
                                log::info!("Installation checked: Game server is up to date: buildid {buildid_after}");
                            } else {
                                log::info!("Updated game server: From buildid {buildid_before} to {buildid_after}");
                            }
                        }
                    }

                    let carbon_installation_checksum: String = match install_or_update_carbon(&config).await {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!("Installing or updating Carbon Modding Framework failed: {err}");
                            Self::request_termination(ctx.tx_activate.clone()).await;
                            break 'loop_transitions;
                        }
                    };
                    log::info!("Carbon Modding Framework installed or updated: SHA256: {carbon_installation_checksum}");

                    let startup_script: String = match generate_game_server_startup_script(&config).await {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!("Failed to generate game server startup script: {err}");
                            Self::request_termination(ctx.tx_activate.clone()).await;
                            break 'loop_transitions;
                        }
                    };

                    Self::InstalledAndConfigured {
                        game_meta: rustctl_common::snapshot::GameServerMetaExposed { buildid: buildid_after },
                        ctx,
                        startup_script,
                    }
                }

                Self::InstalledAndConfigured {
                    ctx,
                    game_meta,
                    startup_script,
                } => {
                    let cfg = ctx.cfg_client.get_config().await;
                    let mut command = tokio::process::Command::new(startup_script);

                    /*
                     * Spawn the game server in a new process group so that
                     * the whole child process tree can be terminated via the
                     * group. The process tree as a whole looks something like
                     * the following:
                     *
                     * 1. RUSTCTL -- this app (native executable)
                     * 2. STARTUP SCRIPT -- a Bash script generated at runtime by RUSTCTL
                     * 3. RUSTDEDICATED -- the actual game server process spawned by the startup script
                     *
                     * SAFETY: Trust be bro.
                     */
                    unsafe {
                        command.pre_exec(|| {
                            /*
                             * Make the child process the leader of a new
                             * process group, where the process group's ID
                             * (pgid) equals to the forked process’s ID (pid).
                             */
                            libc::setpgid(0, 0);
                            Ok(())
                        });
                    }

                    command.current_dir(cfg.fs.root_dir_abs_utf8());
                    command.stdout(std::process::Stdio::piped());
                    command.stderr(std::process::Stdio::piped());

                    let mut process: tokio::process::Child = match command.spawn() {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!(
                                "Failed to spawn game server: {err_fmt}",
                                err_fmt = crate::util::fmt_source_tree(&err),
                            );
                            Self::request_termination(ctx.tx_activate.clone()).await;
                            break 'loop_transitions;
                        }
                    };

                    let (stdout, stderr) = match (process.stdout.take(), process.stderr.take()) {
                        (Some(stdout), Some(stderr)) => (stdout, stderr),
                        _ => {
                            log::error!("Failed to get output handle of game server process",);
                            Self::request_termination(ctx.tx_activate.clone()).await;
                            break 'loop_transitions;
                        }
                    };

                    Self::LaunchingGame {
                        game_meta,
                        process,
                        stdout,
                        stderr,
                        ctx,
                    }
                }

                Self::LaunchingGame {
                    game_meta,
                    process,
                    stdout,
                    stderr,
                    ctx,
                } => {
                    let timeout = std::time::Duration::from_secs(60 * 30); // 30 minutes
                    let mut stdout_reader = tokio::io::BufReader::new(stdout);
                    let mut stderr_reader = tokio::io::BufReader::new(stderr);

                    /*
                     * An "in-actor" channel for signaling readiness based on
                     * spawned game server process's output. Not to be confused
                     * with the other, "inter-actor" readiness signaling
                     * channel!
                     */
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
                        Ok(Ok(_)) => {
                            if let Err(err) = ctx.tx_rconready.send(ReadyForRcon).await {
                                log::error!(
                                    "Inter-actor readiness signaling channel between GSSM and RCON client closed unexpectedly: {err}"
                                );
                                Self::request_termination(ctx.tx_activate.clone()).await;
                                break 'loop_transitions;
                            }
                            Self::GameRunningHealthy {
                                process,
                                game_meta,
                                ctx,
                            }
                        }
                        Ok(Err(err)) => {
                            log::error!(
                                "Readiness signaling channel got teared down while waiting for the signal: {err_fmt}",
                                err_fmt = crate::util::fmt_source_tree(&err)
                            );
                            Self::request_termination(ctx.tx_activate.clone()).await;
                            break 'loop_transitions;
                        }
                        Err(err) => {
                            log::error!(
                                "Game server did not indicate its readiness within timeout of {timeout_secs} seconds: {err_fmt}",
                                timeout_secs = timeout.as_secs(),
                                err_fmt = crate::util::fmt_source_tree(&err)
                            );
                            Self::request_termination(ctx.tx_activate.clone()).await;
                            break 'loop_transitions;
                        }
                    }
                }

                Self::GameRunningHealthy {
                    game_meta,
                    mut process,
                    mut ctx,
                } => {
                    let event: GameCtlEvent = tokio::select! {
                        msg = ctx.rx_command.recv() => {
                            match msg {
                                Some(message) => GameCtlEvent::MessageReceived { message },
                                None => {
                                    log::debug!("Channel for receiving commands closed -- Stopping game server state machine");
                                    break 'loop_transitions;
                                },
                            }
                        }
                        output = process.wait() => {
                            let exit_status: std::process::ExitStatus = match output {
                                Ok(n) => n,
                                Err(err) => {
                                    log::error!("Failed to run game server to termination: {err_fmt}", err_fmt = crate::util::fmt_source_tree(&err));
                                    Self::request_termination(ctx.tx_activate.clone()).await;
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
                                            Self::request_termination(ctx.tx_activate.clone()).await;
                                            break 'loop_transitions;
                                        }
                                    };
                                    log::info!("Sent signal to game server process: {signal}: PID {pid}");
                                    Self::SavingAndClosingGame { process, ctx }
                                }
                                _ => {
                                    log::error!("Ignoring unexpected command: {command:?} for current state");
                                    Self::GameRunningHealthy {
                                        game_meta,
                                        process,
                                        ctx,
                                    }
                                }
                            }
                        }
                        GameCtlEvent::GameProcessTerminated { exit_status } => {
                            let _exit_status: std::process::ExitStatus = exit_status;
                            Self::GameTerminatedUnexpectedly { ctx }
                        }
                    }
                }

                Self::SavingAndClosingGame { mut process, ctx } => {
                    match process.wait().await {
                        Ok(status) => {
                            log::info!("game server process exited with {status}");
                        }
                        Err(err) => {
                            log::error!(
                                "Waiting for game server process to terminate failed: {err_fmt}",
                                err_fmt = crate::util::fmt_source_tree(&err),
                            );
                            Self::request_termination(ctx.tx_activate.clone()).await;
                            break 'loop_transitions;
                        }
                    };

                    Self::GameClosedManually { ctx }
                }

                Self::GameClosedManually { mut ctx } => {
                    let msg = ctx.rx_command.recv().await;
                    if let Some(command) = msg {
                        let command: rustctl_common::command::DownstreamClientMessage = command;
                        match command {
                            rustctl_common::command::DownstreamClientMessage::ServerInstallOrUpdateAndStart => {
                                Self::InstallingUpdates { ctx }
                            }
                            _ => {
                                log::error!("Ignoring unexpected command: {command:?} for current state");
                                Self::GameClosedManually { ctx }
                            }
                        }
                    } else {
                        Self::GameClosedManually { ctx }
                    }
                }

                Self::GameTerminatedUnexpectedly { ctx } => Self::InstallingUpdates { ctx },
            };

            if let Err(err) = self.send_state().await {
                log::debug!(
                    "Channel for sending game server state machine transition to aggregator is closed: {err_fmt} -- Stopping state machine",
                    err_fmt = crate::util::fmt_source_tree(&err),
                );
                break 'loop_transitions;
            }
        }
    }

    async fn send_state(
        &self,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<rustctl_common::snapshot::GameServerStateExposed>> {
        let tx = match self {
            GameServerStateMachine::Init { ctx, .. }
            | GameServerStateMachine::InstallingUpdates { ctx, .. }
            | GameServerStateMachine::InstalledAndConfigured { ctx, .. }
            | GameServerStateMachine::LaunchingGame { ctx, .. }
            | GameServerStateMachine::GameRunningHealthy { ctx, .. }
            | GameServerStateMachine::SavingAndClosingGame { ctx, .. }
            | GameServerStateMachine::GameClosedManually { ctx, .. }
            | GameServerStateMachine::GameTerminatedUnexpectedly { ctx, .. } => &ctx.tx_agg_gss,
        };
        let sendable: rustctl_common::snapshot::GameServerStateExposed = self.into();
        tx.send(sendable).await
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

    /*
     * SAFETY: Trust me bro.
     */
    let pgid: i32 = unsafe { libc::getpgid(pid) };

    if pgid < 0 {
        return Err(ErrorSendingSignal::SendFailed {
            source: nix::Error::last(),
        });
    }

    let pgid: nix::unistd::Pid = nix::unistd::Pid::from_raw(pgid);
    if let Err(source) = nix::sys::signal::killpg(pgid, signal) {
        Err(ErrorSendingSignal::SendFailed { source })
    } else {
        Ok(pgid)
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

impl From<&GameServerStateMachine> for rustctl_common::snapshot::GameServerStateExposed {
    fn from(value: &GameServerStateMachine) -> Self {
        match value {
            GameServerStateMachine::Init { .. } => rustctl_common::snapshot::GameServerStateExposed::Init,
            GameServerStateMachine::InstallingUpdates { .. } => {
                rustctl_common::snapshot::GameServerStateExposed::InstallingUpdates
            }
            GameServerStateMachine::InstalledAndConfigured { game_meta, .. } => {
                rustctl_common::snapshot::GameServerStateExposed::InstalledAndConfigured {
                    game_meta: game_meta.to_owned(),
                }
            }
            GameServerStateMachine::LaunchingGame { game_meta, .. } => {
                rustctl_common::snapshot::GameServerStateExposed::LaunchingGame {
                    game_meta: game_meta.to_owned(),
                }
            }
            GameServerStateMachine::GameRunningHealthy { game_meta, .. } => {
                rustctl_common::snapshot::GameServerStateExposed::GameRunningHealthy {
                    game_meta: game_meta.to_owned(),
                }
            }
            GameServerStateMachine::SavingAndClosingGame { .. } => {
                rustctl_common::snapshot::GameServerStateExposed::SavingAndClosingGame {}
            }
            GameServerStateMachine::GameClosedManually { .. } => {
                rustctl_common::snapshot::GameServerStateExposed::GameClosedManually
            }
            GameServerStateMachine::GameTerminatedUnexpectedly { .. } => {
                rustctl_common::snapshot::GameServerStateExposed::GameTerminatedUnexpectedly
            }
        }
    }
}

pub struct ReadyForRcon;

/// Install or update game server (`RustDedicated`) using installer
/// (`steamcmd`). Return the installed game server's _buildid_ parsed from the
/// installation's associated manifest file.
async fn install_or_update_game_server(config: &crate::storage::Configuration) -> Result<u32, String> {
    let executable: String = config.fs.installer_abs_utf8();
    let working_directory: String = config.fs.root_dir_abs_utf8();

    let mut command = tokio::process::Command::new(&executable);
    command.current_dir(&working_directory);
    command.args(config.get_installer_args());
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());

    let process: tokio::process::Child = match command.spawn() {
        Ok(n) => n,
        Err(err) => {
            return Err(format!("failed to spawn game server installer: {command:?}: {err}"));
        }
    };

    /*
     * TODO: Consider case "offline": Check status code of
     *       installer process exit?
     */
    let _output: std::process::Output = match process.wait_with_output().await {
        Ok(n) => n,
        Err(err) => {
            return Err(format!("failed to run game server installer to termination: {err}"));
        }
    };

    let buildid: Option<u32> = {
        if let Ok(contents) = tokio::fs::read_to_string(config.fs.manifest_abs_utf8()).await {
            extract_buildid_from_buf(&contents)
        } else {
            None
        }
    };

    match buildid {
        Some(n) => Ok(n),
        None => Err(format!(
            r#"failed to extract buildid from manifest "{path}""#,
            path = config.fs.manifest_abs_utf8()
        )),
    }
}

/// Install or update Carbon Modding Framework (https://carbonmod.gg/).
async fn install_or_update_carbon(config: &crate::storage::Configuration) -> Result<String, String> {
    let download_url: &str = &config.carbon_download_url;
    let install_location: &str = &config.fs.root_dir_abs_utf8();
    let temp_dir = std::path::Path::new(&config.fs.temp_dir_abs_utf8()).to_path_buf();

    tokio::fs::create_dir_all(&temp_dir)
        .await
        .map_err(|err| format!("failed to create temporary directory: {err}"))?;

    let archive_path: std::path::PathBuf = temp_dir.join("carbon.tar.gz");

    log::debug!("Downloading Carbon from: {download_url}");
    let output: std::process::Output = tokio::process::Command::new("wget")
        .arg("-O")
        .arg(&archive_path)
        .arg(download_url)
        .output()
        .await
        .map_err(|err| format!("failed to execute wget: {err}"))?;
    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("wget failed: {error_msg}"));
    }

    let metadata: std::fs::Metadata = tokio::fs::metadata(&archive_path)
        .await
        .map_err(|err| format!("failed to get archive metadata: {err}"))?;

    let bytes: u64 = metadata.len();
    if bytes == 0 {
        return Err("downloaded archive is empty".to_string());
    }

    let checksum_output: std::process::Output = tokio::process::Command::new("sha256sum")
        .arg(&archive_path)
        .output()
        .await
        .map_err(|err| format!("failed to calculate SHA256: {err}"))?;
    if !checksum_output.status.success() {
        return Err("failed to calculate SHA256 checksum".to_string());
    }

    let checksum_str = String::from_utf8_lossy(&checksum_output.stdout);
    let sha256: String = checksum_str
        .split_whitespace()
        .next()
        .ok_or("invalid SHA256 output format")?
        .to_string();
    log::info!(
        r#"Downloaded Carbon Modding Framework: {bytes} bytes (~{kibibytes} KiB): "{archive_path}" (SHA256: {sha256})"#,
        kibibytes = bytes / 1024,
        archive_path = archive_path.to_string_lossy(),
    );

    log::debug!("Extracting Carbon Modding Framework to: {install_location}");
    tokio::fs::create_dir_all(install_location)
        .await
        .map_err(|err| format!("failed to create install directory: {err}"))?;

    let extract_output: std::process::Output = tokio::process::Command::new("tar")
        .arg("-xzf")
        .arg(&archive_path)
        .arg("-C")
        .arg(install_location)
        .output()
        .await
        .map_err(|err| format!("failed to execute tar: {err}"))?;
    if !extract_output.status.success() {
        let error_msg = String::from_utf8_lossy(&extract_output.stderr);
        return Err(format!("tar extraction failed: {error_msg}"));
    }

    let carbon_script = std::path::Path::new(install_location).join("carbon.sh");
    let carbon_dir = std::path::Path::new(install_location).join("carbon");
    if !tokio::fs::try_exists(&carbon_script).await.unwrap_or(false) {
        return Err("extraction failed: carbon.sh not found".to_string());
    }
    if !tokio::fs::try_exists(&carbon_dir).await.unwrap_or(false) {
        return Err("extraction failed: carbon directory not found".to_string());
    }

    let chmod_output: std::process::Output = tokio::process::Command::new("chmod")
        .arg("+x")
        .arg(&carbon_script)
        .output()
        .await
        .map_err(|err| format!("failed to make carbon.sh executable: {err}"))?;
    if !chmod_output.status.success() {
        return Err(format!(
            "failed to make carbon.sh executable: chmod {status}",
            status = chmod_output.status,
        ));
    }

    Ok(sha256)
}

/// Generate a Bash script to be used as game server's entry point.
async fn generate_game_server_startup_script(config: &crate::storage::Configuration) -> Result<String, String> {
    let startup_script: &str = &config.fs.startup_script_abs_utf8();

    let script_content: String = format!(
        r#"#!/bin/bash

set -e

export LD_LIBRARY_PATH="{game_server_libs}"

source {carbon_env_init}

{game_executable} \
    -batchmode \
    +server.hostname "{game_name}" \
    +server.description "{game_description}" \
    +server.url "{game_url_home}" \
    +server.headerimage "{game_url_header}" \
    +server.logoimage "{game_url_logo}" \
    +server.maxplayers "42" \
    +server.premium "1" \
    +server.identity "{game_instance_id}" \
    +rcon.port "{rcon_port}" \
    +rcon.web "1" \
    +rcon.password "{rcon_password}" \
    +server.worldsize "{game_world_size}" \
    +server.seed "{game_world_seed}"
"#,
        carbon_env_init = config.fs.carbon_init_script_abs_utf8(),
        game_executable = config.fs.game_abs_utf8(),
        game_name = config.game_name,
        game_description = config.game_description,
        game_url_home = config.game_url_home,
        game_url_header = config.game_url_header,
        game_url_logo = config.game_url_logo,
        game_server_libs = config.fs.root_dir_abs_utf8(),
        game_instance_id = config.game_instance_id,
        rcon_port = config.rcon_port,
        rcon_password = config.rcon_password,
        game_world_size = config.game_world_size,
        game_world_seed = config.game_world_seed,
    );

    tokio::fs::write(startup_script, &script_content)
        .await
        .map_err(|err| format!("failed to write startup script: {err}"))?;

    let chmod_output: std::process::Output = tokio::process::Command::new("chmod")
        .arg("+x")
        .arg(startup_script)
        .output()
        .await
        .map_err(|err| format!("failed to make startup script executable: {err}"))?;
    if !chmod_output.status.success() {
        return Err(format!(
            "failed to make startup script executable: chmod {status}",
            status = chmod_output.status,
        ));
    }

    Ok(startup_script.to_string())
}

/// Returns process IDs (PIDs) of workloads running already: game server
/// installer (`steamcmd`), the game server itself (`RustDedicated`)...
async fn is_running_already(config: &crate::storage::Configuration) -> Vec<u32> {
    let mut running: Vec<u32> = Vec::new();

    /*
     * Check "installer".
     */
    {
        if let Some(executable) = std::path::Path::new(&config.fs.installer_abs_utf8()).file_name() {
            if let Ok(output) = tokio::process::Command::new("pgrep").arg(executable).output().await {
                if output.status.success() {
                    if let Ok(stdout) = String::from_utf8(output.stdout) {
                        if let Ok(pid) = stdout.trim().parse::<u32>() {
                            running.push(pid);
                        }
                    }
                }
            }
        }
    }

    /*
     * Check "generated launcher script".
     */
    {
        if let Some(executable) = std::path::Path::new(&config.fs.startup_script_abs_utf8()).file_name() {
            if let Ok(output) = tokio::process::Command::new("pgrep")
                .arg("-f")
                .arg(executable)
                .output()
                .await
            {
                if output.status.success() {
                    if let Ok(stdout) = String::from_utf8(output.stdout) {
                        if let Ok(pid) = stdout.trim().parse::<u32>() {
                            running.push(pid);
                        }
                    }
                }
            }
        }
    }

    /*
     * Check "game server".
     */
    {
        if let Some(executable) = std::path::Path::new(&config.fs.game_abs_utf8()).file_name() {
            if let Ok(output) = tokio::process::Command::new("pgrep").arg(executable).output().await {
                if output.status.success() {
                    if let Ok(stdout) = String::from_utf8(output.stdout) {
                        if let Ok(pid) = stdout.trim().parse::<u32>() {
                            running.push(pid);
                        }
                    }
                }
            }
        }
    }

    running
}
