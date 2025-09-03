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
                Self::Init { ctx } => Self::InstallingUpdates { ctx },

                /*
                 * Install or update `RustDedicated` using `steamcmd`.
                 */
                Self::InstallingUpdates { ctx } => {
                    let config: crate::storage::Configuration = ctx.cfg_client.get_config().await;

                    let buildid_before: Option<u32> = {
                        if let Ok(contents) = tokio::fs::read_to_string(config.game_manifest).await {
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

                    let carbon_installed: () = match install_or_update_carbon(&config).await {
                        Ok(n) => n,
                        Err(_) => todo!(),
                    };

                    Self::InstalledAndConfigured {
                        game_meta: rustctl_common::snapshot::GameServerMetaExposed { buildid: buildid_after },
                        ctx,
                    }
                }

                Self::InstalledAndConfigured { ctx, game_meta } => {
                    let cfg = ctx.cfg_client.get_config().await;
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
    let mut command = tokio::process::Command::new(config.installer_exe);
    command.current_dir(config.game_server_root);
    command.args(config.get_installer_args());
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());

    let process: tokio::process::Child = match command.spawn() {
        Ok(n) => n,
        Err(err) => {
            return Err(format!("failed to spawn game server installer: {err}"));
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
        if let Ok(contents) = tokio::fs::read_to_string(config.game_manifest).await {
            extract_buildid_from_buf(&contents)
        } else {
            None
        }
    };

    match buildid {
        Some(n) => Ok(n),
        None => Err(format!(
            r#"failed to extract buildid from manifest "{path}""#,
            path = config.game_manifest
        )),
    }
}

/// Install or update Carbon Modding Framework (https://carbonmod.gg/).
async fn install_or_update_carbon(config: &crate::storage::Configuration) -> Result<(), String> {
    let download_url: &str = &config.carbon_download_url; // e.g. "https://github.com/CarbonCommunity/Carbon/releases/download/production_build/Carbon.Linux.Minimal.tar.gz"

    /*
     * TODO:
     *
     * 1. Download Carbon Modding Framework, i.e. a `.tar.gz` file, from `download_url`.
     *
     * 2. Extract the `.tar.gz` file. Example of contents:
     *
     *    ```
     *    $ wget https://github.com/CarbonCommunity/Carbon/releases/download/production_build/Carbon.Linux.Minimal.tar.gz
     *
     *    $ ls -l
     *    Carbon.Linux.Minimal.tar.gz
     *
     *    $ mkdir extracted
     *
     *    $ tar -xzf Carbon.Linux.Minimal.tar.gz -C extracted
     *
     *    $ tree extracted/
     *    extracted/
     *    ├── carbon
     *    │   ├── configs
     *    │   ├── data
     *    │   ├── extensions
     *    │   ├── managed
     *    │   │   ├── Carbon.Bootstrap.dll
     *    │   │   ├── Carbon.Common.dll
     *    │   │   ├── Carbon.Compat.dll
     *    │   │   ├── Carbon.dll
     *    │   │   ├── Carbon.Preloader.dll
     *    │   │   ├── Carbon.Profiler.dll
     *    │   │   ├── Carbon.SDK.dll
     *    │   │   ├── Carbon.Startup.dll
     *    │   │   ├── Carbon.Test.dll
     *    │   │   ├── Carbon.UniTask.dll
     *    │   │   ├── hooks
     *    │   │   │   ├── Carbon.Hooks.Base.dll
     *    │   │   │   ├── Carbon.Hooks.Community.dll
     *    │   │   │   └── Carbon.Hooks.Oxide.dll
     *    │   │   ├── lib
     *    │   │   │   ├── 0Harmony.dll
     *    │   │   │   ├── AsmResolver.dll
     *    │   │   │   ├── AsmResolver.DotNet.dll
     *    │   │   │   ├── AsmResolver.PE.dll
     *    │   │   │   ├── AsmResolver.PE.File.dll
     *    │   │   │   ├── Ben.Demystifier.dll
     *    │   │   │   ├── BouncyCastle.Crypto.dll
     *    │   │   │   ├── EntityFramework.dll
     *    │   │   │   ├── EntityFramework.SqlServer.dll
     *    │   │   │   ├── Google.Protobuf.dll
     *    │   │   │   ├── Humanizer.dll
     *    │   │   │   ├── ICSharpCode.Decompiler.dll
     *    │   │   │   ├── K4os.Compression.LZ4.dll
     *    │   │   │   ├── K4os.Compression.LZ4.Streams.dll
     *    │   │   │   ├── K4os.Hash.xxHash.dll
     *    │   │   │   ├── Microsoft.Bcl.AsyncInterfaces.dll
     *    │   │   │   ├── Microsoft.CodeAnalysis.CSharp.dll
     *    │   │   │   ├── Microsoft.CodeAnalysis.CSharp.Workspaces.dll
     *    │   │   │   ├── Microsoft.CodeAnalysis.dll
     *    │   │   │   ├── Microsoft.CodeAnalysis.Workspaces.dll
     *    │   │   │   ├── Mono.Cecil.dll
     *    │   │   │   ├── Mono.Cecil.Mdb.dll
     *    │   │   │   ├── Mono.Cecil.Pdb.dll
     *    │   │   │   ├── Mono.Cecil.Rocks.dll
     *    │   │   │   ├── Mono.Data.Sqlite.dll
     *    │   │   │   ├── MySql.Data.dll
     *    │   │   │   ├── protobuf-net.Core.dll
     *    │   │   │   ├── protobuf-net.dll
     *    │   │   │   ├── QRCoder.dll
     *    │   │   │   ├── Roslynator.Core.dll
     *    │   │   │   ├── Roslynator.CSharp.dll
     *    │   │   │   ├── SharpCompress.dll
     *    │   │   │   ├── System.Buffers.dll
     *    │   │   │   ├── System.Collections.Immutable.dll
     *    │   │   │   ├── System.Composition.AttributedModel.dll
     *    │   │   │   ├── System.Composition.Convention.dll
     *    │   │   │   ├── System.Composition.Hosting.dll
     *    │   │   │   ├── System.Composition.Runtime.dll
     *    │   │   │   ├── System.Composition.TypedParts.dll
     *    │   │   │   ├── System.Data.SQLite.dll
     *    │   │   │   ├── System.Data.SQLite.EF6.dll
     *    │   │   │   ├── System.Data.SQLite.Linq.dll
     *    │   │   │   ├── System.IO.Pipelines.dll
     *    │   │   │   ├── System.Memory.dll
     *    │   │   │   ├── System.Numerics.Vectors.dll
     *    │   │   │   ├── System.Reflection.Metadata.dll
     *    │   │   │   ├── System.Runtime.CompilerServices.Unsafe.dll
     *    │   │   │   ├── System.Text.Encoding.CodePages.dll
     *    │   │   │   ├── System.Text.Encodings.Web.dll
     *    │   │   │   ├── System.Text.Json.dll
     *    │   │   │   ├── System.Threading.Channels.dll
     *    │   │   │   ├── System.Threading.Tasks.Extensions.dll
     *    │   │   │   ├── System.ValueTuple.dll
     *    │   │   │   ├── websocket-sharp.dll
     *    │   │   │   ├── x64
     *    │   │   │   │   └── SQLite.Interop.dll
     *    │   │   │   ├── x86
     *    │   │   │   │   └── SQLite.Interop.dll
     *    │   │   │   └── ZstdSharp.dll
     *    │   │   └── modules
     *    │   ├── native
     *    │   │   └── libCarbonNative.so
     *    │   ├── plugins
     *    │   └── tools
     *    │       └── environment.sh
     *    ├── carbon.sh
     *    ├── Carbon.targets
     *    └── libdoorstop.so
     *    ```
     */

    todo!();
}
