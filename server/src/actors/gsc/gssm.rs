//! Game Server State Machine (GSSM).

pub struct Context {
    pub ctoken: tokio_util::sync::CancellationToken,
    pub tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

    pub skip: bool,

    pub db_client: crate::actors::database::client::Client,

    pub rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
    /// "GSS" = "Game Server State"
    pub tx_agg_gss: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,
    pub tx_rconready: tokio::sync::mpsc::Sender<crate::actors::gsc::gssm::ReadyForRcon>,
    pub rx_buildid: tokio::sync::mpsc::Receiver<crate::actors::game_monitor::GameBuildIDUpdate>,
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
    #[allow(clippy::too_many_arguments)]
    pub fn init(
        ctoken: tokio_util::sync::CancellationToken,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

        skip: bool,

        db_client: crate::actors::database::client::Client,

        rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
        tx_agg_gss: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,
        tx_rconready: tokio::sync::mpsc::Sender<crate::actors::gsc::gssm::ReadyForRcon>,
        rx_buildid: tokio::sync::mpsc::Receiver<crate::actors::game_monitor::GameBuildIDUpdate>,
    ) -> Self {
        Self::Init {
            ctx: Context {
                ctoken,
                tx_activate,

                skip,

                db_client,

                rx_command,
                tx_agg_gss,
                tx_rconready,
                rx_buildid,
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
                Self::Init { mut ctx } => {
                    let running_already: Vec<u32> = is_running_already().await;
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

                    /*
                     * TODO: If the game server is being started on the first Thursday of the month,
                     *       assume it might be _the_ monthly "forced" content update, and so wipe
                     *       the map (and blueprints?) unless already wiped on the same day.
                     */
                    let latest_wipe: Option<crate::data::schema::Wipe> = ctx.db_client.read_latest_wipe().await;
                    dbg!(latest_wipe);

                    Self::InstallingUpdates { ctx }
                }

                /*
                 * Install or update `RustDedicated` using `steamcmd`.
                 */
                Self::InstallingUpdates { mut ctx } => {
                    let config: rustctl_backend::GameParameters = ctx.db_client.read_current_config().await;

                    /*
                     * Install/update game server.
                     */
                    let buildid_before: Option<crate::steam::BuildID> =
                        crate::steam::BuildID::from_existing_installation_manifest(
                            rustctl_backend::constants::paths::MANIFEST,
                        )
                        .await;

                    #[allow(clippy::needless_late_init)]
                    let buildid_after: crate::steam::BuildID;
                    if ctx.skip {
                        buildid_after = match buildid_before {
                            Some(ref buildid_before) => {
                                log::warn!(
                                    "Skipping updating game server -- Existing installation build ID: {buildid_before}"
                                );
                                buildid_before.clone()
                            }
                            None => {
                                log::error!(
                                    "No existing installation found, and installation skipped -- Cannot start game server!"
                                );
                                Self::request_termination(ctx.tx_activate.clone()).await;
                                break 'loop_transitions;
                            }
                        }
                    } else {
                        buildid_after = match crate::steam::RustDedicated::install(&config).await {
                            Ok(buildid_installed) => {
                                if let Some(buildid_before) = buildid_before {
                                    if buildid_before == buildid_installed {
                                        log::info!(
                                            "Game server installation checked: Already up-to-date: Build ID: {buildid_installed}"
                                        );
                                    } else {
                                        log::info!(
                                            "Game server updated: From build ID {buildid_before} to build ID {buildid_installed}"
                                        );
                                    }
                                } else {
                                    log::info!("Game server installed: Build ID {buildid_installed}");
                                }
                                buildid_installed
                            }
                            Err(err) => {
                                log::error!("Installing game server failed: {err}");
                                Self::request_termination(ctx.tx_activate.clone()).await;
                                break 'loop_transitions;
                            }
                        };
                    }

                    /*
                     * Install/update modding framework.
                     */
                    if !ctx.skip {
                        let carbon_installation_checksum: String = match install_or_update_carbon(&config).await {
                            Ok(n) => n,
                            Err(err) => {
                                log::error!("Installing or updating Carbon Modding Framework failed: {err}");
                                Self::request_termination(ctx.tx_activate.clone()).await;
                                break 'loop_transitions;
                            }
                        };
                        log::info!(
                            "Carbon Modding Framework installed or updated: SHA256: {carbon_installation_checksum}"
                        );
                    } else {
                        log::warn!("Skipping installing/updating modding framework");
                    }

                    /*
                     * Instrument game server by installing a custom plugin.
                     */
                    if let Err(err) = install_plugin().await {
                        todo!("{err}");
                    }

                    let startup_script: String = match generate_game_server_startup_script(&config).await {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!("Failed to generate game server startup script: {err}");
                            Self::request_termination(ctx.tx_activate.clone()).await;
                            break 'loop_transitions;
                        }
                    };

                    Self::InstalledAndConfigured {
                        game_meta: rustctl_common::snapshot::GameServerMetaExposed {
                            buildid: buildid_after.into(),
                        },
                        ctx,
                        startup_script,
                    }
                }

                Self::InstalledAndConfigured {
                    ctx,
                    game_meta,
                    startup_script,
                } => {
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

                    command.current_dir(rustctl_backend::constants::paths::ROOT_DIR);
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

                    /*
                     * Hook the spawned process group to the termination mechanism.
                     */
                    if let Some(pid) = process.id() {
                        /*
                         * SAFETY: Trust me bro.
                         */
                        let pgid: i32 = unsafe { libc::getpgid(pid as i32) };
                        let pgid: nix::unistd::Pid = nix::unistd::Pid::from_raw(pgid);

                        let ctoken = ctx.ctoken.child_token();
                        let _term_job = tokio::spawn(async move {
                            ctoken.cancelled().await;
                            log::info!(
                                "Sending termination signal to game server process group: PID {pid}, PGID {pgid}"
                            );
                            _ = nix::sys::signal::killpg(pgid, nix::sys::signal::Signal::SIGTERM);
                        });
                    }

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
                                    if trimmed_line.contains("SteamServer Connected")
                                        && let Some(sender) = tx.take()
                                    {
                                        let _ = sender.send(());
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
                        msg = ctx.rx_buildid.recv() => {
                            if let Some(update) = msg {
                                let update: crate::actors::game_monitor::GameBuildIDUpdate = update;
                                GameCtlEvent::BuildIDUpdate { update }
                            } else {
                                log::debug!("Channel for receiving build ID updates closed -- Stopping game server state machine");
                                break 'loop_transitions;
                            }
                        },
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
                                    let signal = nix::sys::signal::Signal::SIGTERM;
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

                        GameCtlEvent::BuildIDUpdate { update } => {
                            let buildid_current: crate::steam::BuildID = crate::steam::BuildID::new(game_meta.buildid);
                            let buildid_latest_avail: crate::steam::BuildID = update.latest_available_build_id;
                            let players_online: u16 = update.players_online;

                            if buildid_current != buildid_latest_avail {
                                if players_online == 0 {
                                    /*
                                     * Case there's an update available and there are no players
                                     * on the server. Either we have a "forced update" (_the_
                                     * monthly content update) that causes clients be unable to
                                     * connect, or some optional update which we might as well
                                     * install since no one is online!
                                     */
                                    log::info!(
                                        "Update available and no players on the server: Current build ID: {buildid_current}, latest available: {buildid_latest_avail} -- Terminating game server!"
                                    );
                                    let signal = nix::sys::signal::Signal::SIGTERM;
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
                                } else {
                                    /*
                                     * Case there's an update available, yet there are also players on the
                                     * server. Presumably this implies that the update is optional and thus we
                                     * may simply ignore it for now! (The update shall be installed at a later
                                     * check when there are no players online!)
                                     */
                                    log::debug!(
                                        "Update available yet there are {players_online} players on the server: Current build ID: {buildid_current}, latest available: {buildid_latest_avail}"
                                    );
                                    Self::GameRunningHealthy {
                                        game_meta,
                                        process,
                                        ctx,
                                    }
                                }
                            } else {
                                /*
                                 * Case latest avail version matches current, i.e. already
                                 * up-to-date: Nothing to do!
                                 */
                                Self::GameRunningHealthy {
                                    game_meta,
                                    process,
                                    ctx,
                                }
                            }
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

enum GameCtlEvent {
    MessageReceived {
        message: rustctl_common::command::DownstreamClientMessage,
    },

    GameProcessTerminated {
        exit_status: std::process::ExitStatus,
    },

    BuildIDUpdate {
        update: crate::actors::game_monitor::GameBuildIDUpdate,
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

async fn install_plugin() -> Result<(), String> {
    let plugin_contents: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../carbon/plugin.cs"));

    tokio::fs::write(rustctl_backend::constants::paths::PLUGIN, plugin_contents)
        .await
        .map_err(|e| format!("Failed to write plugin file: {e}"))?;

    Ok(())
}

/// Install or update Carbon Modding Framework (https://carbonmod.gg/).
async fn install_or_update_carbon(config: &rustctl_backend::GameParameters) -> Result<String, String> {
    let download_url: &str = &config.carbon_download_url;

    log::debug!("Downloading Carbon from: {download_url}");
    let output: std::process::Output = tokio::process::Command::new("wget")
        .arg("-O")
        .arg(rustctl_backend::constants::paths::TMP_ARCHIVE)
        .arg(download_url)
        .output()
        .await
        .map_err(|err| format!("failed to execute wget: {err}"))?;
    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("wget failed: {error_msg}"));
    }

    let metadata: std::fs::Metadata = tokio::fs::metadata(rustctl_backend::constants::paths::TMP_ARCHIVE)
        .await
        .map_err(|err| format!("failed to get archive metadata: {err}"))?;

    let bytes: u64 = metadata.len();
    if bytes == 0 {
        return Err("downloaded archive is empty".to_string());
    }

    let checksum_output: std::process::Output = tokio::process::Command::new("sha256sum")
        .arg(rustctl_backend::constants::paths::TMP_ARCHIVE)
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
        archive_path = rustctl_backend::constants::paths::TMP_ARCHIVE,
    );

    log::debug!(
        "Extracting Carbon Modding Framework to: {install_location}",
        install_location = rustctl_backend::constants::paths::ROOT_DIR,
    );

    let extract_output: std::process::Output = tokio::process::Command::new("tar")
        .arg("-xzf")
        .arg(rustctl_backend::constants::paths::TMP_ARCHIVE)
        .arg("-C")
        .arg(rustctl_backend::constants::paths::ROOT_DIR)
        .output()
        .await
        .map_err(|err| format!("failed to execute tar: {err}"))?;
    if !extract_output.status.success() {
        let error_msg = String::from_utf8_lossy(&extract_output.stderr);
        return Err(format!("tar extraction failed: {error_msg}"));
    }

    Ok(sha256)
}

/// Generate a Bash script to be used as game server's entry point.
async fn generate_game_server_startup_script(config: &rustctl_backend::GameParameters) -> Result<String, String> {
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
        carbon_env_init = rustctl_backend::constants::paths::CARBON_INIT,
        game_executable = rustctl_backend::constants::paths::GAME,
        game_name = config.game_name,
        game_description = config.game_description,
        game_url_home = config.game_url_home,
        game_url_header = config.game_url_header,
        game_url_logo = config.game_url_logo,
        game_server_libs = rustctl_backend::constants::paths::ROOT_DIR,
        game_instance_id = rustctl_backend::constants::names::GAME_INSTANCE_ID,
        rcon_port = config.rcon_port,
        rcon_password = config.rcon_password,
        game_world_size = config.game_world_size,
        game_world_seed = config.game_world_seed,
    );

    tokio::fs::write(rustctl_backend::constants::paths::STARTUP, &script_content)
        .await
        .map_err(|err| {
            format!(
                "failed to write startup script {startup_script}: {err}",
                startup_script = rustctl_backend::constants::paths::STARTUP,
            )
        })?;

    let chmod_output: std::process::Output = tokio::process::Command::new("chmod")
        .arg("+x")
        .arg(rustctl_backend::constants::paths::STARTUP)
        .output()
        .await
        .map_err(|err| format!("failed to make startup script executable: {err}"))?;
    if !chmod_output.status.success() {
        return Err(format!(
            "failed to make startup script executable: chmod {status}",
            status = chmod_output.status,
        ));
    }

    Ok(rustctl_backend::constants::paths::STARTUP.to_string())
}

/// Returns process IDs (PIDs) of workloads running already: game server
/// installer (`steamcmd`), the game server itself (`RustDedicated`)...
async fn is_running_already() -> Vec<u32> {
    let mut running: Vec<u32> = Vec::new();

    /*
     * Check "installer".
     */
    {
        if let Some(executable) = std::path::Path::new(rustctl_backend::constants::paths::INSTALLER).file_name()
            && let Ok(output) = tokio::process::Command::new("pgrep").arg(executable).output().await
            && output.status.success()
            && let Ok(stdout) = String::from_utf8(output.stdout)
            && let Ok(pid) = stdout.trim().parse::<u32>()
        {
            running.push(pid);
        }
    }

    /*
     * Check "generated launcher script".
     */
    {
        if let Some(executable) = std::path::Path::new(rustctl_backend::constants::paths::STARTUP).file_name()
            && let Ok(output) = tokio::process::Command::new("pgrep")
                .arg("-f")
                .arg(executable)
                .output()
                .await
            && output.status.success()
            && let Ok(stdout) = String::from_utf8(output.stdout)
            && let Ok(pid) = stdout.trim().parse::<u32>()
        {
            running.push(pid);
        }
    }

    /*
     * Check "game server".
     */
    {
        if let Some(executable) = std::path::Path::new(rustctl_backend::constants::paths::GAME).file_name()
            && let Ok(output) = tokio::process::Command::new("pgrep").arg(executable).output().await
            && output.status.success()
            && let Ok(stdout) = String::from_utf8(output.stdout)
            && let Ok(pid) = stdout.trim().parse::<u32>()
        {
            running.push(pid);
        }
    }

    running
}
